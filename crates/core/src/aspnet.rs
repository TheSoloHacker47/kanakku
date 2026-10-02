//! Reading ASP.NET WebForms pages: the form's hidden state, its drop-downs, and GridView tables.
//!
//! Such pages are driven by POSTs that carry the page's `__VIEWSTATE` back to the server, with
//! `__EVENTTARGET` and `__EVENTARGUMENT` naming what was clicked (`gvState`, `Select$3`). Sulekha's
//! public plan view works this way. Everything here is plain string work, tested natively.

/// The state a postback must send back.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Form {
    /// Hidden inputs, in page order.
    pub hidden: Vec<(String, String)>,
    pub selects: Vec<Select>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Select {
    pub name: String,
    /// `(value, label, selected)`.
    pub options: Vec<(String, String, bool)>,
}

impl Select {
    /// The value the browser would send: the selected option, or the first.
    pub fn current(&self) -> Option<&str> {
        self.options.iter().find(|(_, _, selected)| *selected).or(self.options.first()).map(|(v, _, _)| v.as_str())
    }
}

impl Form {
    pub fn parse(html: &str) -> Form {
        let mut form = Form::default();
        for tag in tags(html, "input") {
            if attr(tag, "type").is_some_and(|t| t.eq_ignore_ascii_case("hidden")) {
                if let Some(name) = attr(tag, "name") {
                    form.hidden.push((name, attr(tag, "value").unwrap_or_default()));
                }
            }
        }
        let lower = html.to_ascii_lowercase();
        let mut from = 0;
        while let Some(start) = lower[from..].find("<select").map(|i| i + from) {
            let Some(end) = lower[start..].find("</select>").map(|i| i + start) else { break };
            let block = &html[start..end];
            let open_end = block.find('>').unwrap_or(block.len());
            if let Some(name) = attr(&block[..open_end], "name") {
                let mut options = Vec::new();
                let block_lower = block.to_ascii_lowercase();
                let mut at = 0;
                while let Some(o) = block_lower[at..].find("<option").map(|i| i + at) {
                    let tag_end = block[o..].find('>').map(|i| i + o).unwrap_or(block.len());
                    let label_end = block_lower[tag_end..].find("</option").map(|i| i + tag_end).unwrap_or(block.len());
                    let tag = &block[o..tag_end];
                    let value = attr(tag, "value").unwrap_or_default();
                    let label = text(&block[(tag_end + 1).min(label_end)..label_end]);
                    let selected = tag.to_ascii_lowercase().contains("selected");
                    options.push((value, label, selected));
                    at = label_end;
                }
                form.selects.push(Select { name, options });
            }
            from = end;
        }
        form
    }

    pub fn select(&self, name: &str) -> Option<&Select> {
        self.selects.iter().find(|s| s.name == name)
    }

    /// The urlencoded body of a postback for `target` and `argument`, with `choices` overriding
    /// drop-down values (for a drop-down whose change is itself the postback).
    pub fn postback(&self, target: &str, argument: &str, choices: &[(&str, &str)]) -> String {
        let mut pairs: Vec<(String, String)> = Vec::new();
        for (name, value) in &self.hidden {
            let value = match name.as_str() {
                "__EVENTTARGET" => target.to_string(),
                "__EVENTARGUMENT" => argument.to_string(),
                _ => value.clone(),
            };
            pairs.push((name.clone(), value));
        }
        for wanted in ["__EVENTTARGET", "__EVENTARGUMENT"] {
            if !pairs.iter().any(|(n, _)| n == wanted) {
                pairs.push((wanted.to_string(), if wanted == "__EVENTTARGET" { target } else { argument }.to_string()));
            }
        }
        for select in &self.selects {
            let chosen = choices.iter().find(|(n, _)| *n == select.name).map(|(_, v)| *v).or(select.current());
            if let Some(value) = chosen {
                pairs.push((select.name.clone(), value.to_string()));
            }
        }
        pairs.iter().map(|(n, v)| format!("{}={}", form_encode(n), form_encode(v))).collect::<Vec<_>>().join("&")
    }
}

/// A GridView table: its header, its data rows, and the pages its pager offers.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Grid {
    pub headers: Vec<String>,
    pub rows: Vec<GridRow>,
    /// Page numbers the pager links to (`Page$N`), not counting the current page.
    pub pages: Vec<u32>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct GridRow {
    pub cells: Vec<String>,
    /// Postbacks the row links to, as `(target, argument)`.
    pub postbacks: Vec<(String, String)>,
}

impl Grid {
    /// The index of the first column whose header contains any of `words`, ignoring case.
    pub fn column(&self, words: &[&str]) -> Option<usize> {
        self.headers.iter().position(|h| {
            let h = h.to_ascii_uppercase();
            words.iter().any(|w| h.contains(&w.to_ascii_uppercase()))
        })
    }
}

/// The table with `id`, if the page has it.
pub fn grid(html: &str, id: &str) -> Option<Grid> {
    let lower = html.to_ascii_lowercase();
    let needle = format!("id=\"{}\"", id.to_ascii_lowercase());
    let id_at = lower.find(&needle)?;
    let start = lower[..id_at].rfind("<table")?;
    let end = matching_close(&lower, start, "table")?;
    let table = &html[start..end];
    let table_lower = &lower[start..end];

    let mut grid = Grid::default();
    for (row_start, row_end) in top_level(table_lower, "tr") {
        let row = &table[row_start..row_end];
        let row_lower = &table_lower[row_start..row_end];
        let postbacks = postbacks(row);
        // The pager is a row holding a nested table of Page$N links.
        if row_lower.contains("<table") && postbacks.iter().all(|(_, arg)| arg.starts_with("Page$")) {
            grid.pages.extend(postbacks.iter().filter_map(|(_, arg)| arg.strip_prefix("Page$")?.parse::<u32>().ok()));
            continue;
        }
        let header = row_lower.contains("<th");
        let cells: Vec<String> = top_level(row_lower, "th")
            .into_iter()
            .chain(top_level(row_lower, "td"))
            .map(|(s, e)| {
                let inner_start = row[s..e].find('>').map(|i| s + i + 1).unwrap_or(e);
                let inner_end = row_lower[..e].rfind("</").filter(|&i| i >= inner_start).unwrap_or(e);
                text(&row[inner_start..inner_end])
            })
            .collect();
        if header && grid.headers.is_empty() {
            grid.headers = cells;
        } else if !cells.is_empty() {
            grid.rows.push(GridRow { cells, postbacks });
        }
    }
    grid.pages.sort_unstable();
    grid.pages.dedup();
    Some(grid)
}

/// `__doPostBack('target','argument')` calls in a fragment, in order, HTML-quoted or not.
pub fn postbacks(fragment: &str) -> Vec<(String, String)> {
    let decoded = fragment.replace("&#39;", "'").replace("&quot;", "\"");
    let mut out = Vec::new();
    let mut rest = decoded.as_str();
    while let Some(i) = rest.find("__doPostBack(") {
        rest = &rest[i + "__doPostBack(".len()..];
        let args: Vec<&str> = rest.split(')').next().unwrap_or_default().splitn(2, ',').collect();
        if args.len() == 2 {
            let unquote = |s: &str| s.trim().trim_matches(|c| c == '\'' || c == '"').to_string();
            out.push((unquote(args[0]), unquote(args[1])));
        }
    }
    out
}

/// A number as Sulekha and similar pages print it: `1,23,456.78`, possibly empty.
pub fn number(s: &str) -> Option<f64> {
    let cleaned: String = s.chars().filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
    if cleaned.is_empty() {
        None
    } else {
        cleaned.parse().ok()
    }
}

/// Start and end (exclusive, after the closing tag) of each `name` element directly inside the
/// fragment, skipping ones inside nested tables.
fn top_level(lower: &str, name: &str) -> Vec<(usize, usize)> {
    let open = format!("<{name}");
    let mut out = Vec::new();
    // Start after the fragment's own opening tag; `depth` counts nested tables entered.
    let mut at = lower.find('>').map(|i| i + 1).unwrap_or(0);
    let mut depth = 0i32;
    while at < lower.len() {
        let next_table = lower[at..].find("<table").map(|i| i + at);
        let next_close = lower[at..].find("</table").map(|i| i + at);
        let next_open = lower[at..].find(&open).map(|i| i + at).filter(|&i| is_tag_start(lower, i, name));
        let candidates = [next_table, next_close, next_open];
        let Some(next) = candidates.iter().flatten().min().copied() else { break };
        if Some(next) == next_table && Some(next) != next_open {
            depth += 1;
            at = next + 6;
        } else if Some(next) == next_close {
            depth -= 1;
            at = next + 7;
        } else {
            let Some(end) = matching_close(lower, next, name) else { break };
            if depth == 0 {
                out.push((next, end));
            }
            at = end;
        }
    }
    out
}

fn is_tag_start(lower: &str, i: usize, name: &str) -> bool {
    lower.as_bytes().get(i + 1 + name.len()).is_some_and(|b| matches!(b, b' ' | b'>' | b'\t' | b'\n' | b'\r' | b'/'))
}

/// The end (after `</name>`) of the element opening at `start`, counting nested ones.
fn matching_close(lower: &str, start: usize, name: &str) -> Option<usize> {
    let open = format!("<{name}");
    let close = format!("</{name}");
    let mut depth = 0;
    let mut at = start;
    loop {
        let o = lower[at..].find(&open).map(|i| i + at).filter(|&i| is_tag_start(lower, i, name));
        let c = lower[at..].find(&close).map(|i| i + at)?;
        match o {
            Some(o) if o < c => {
                depth += 1;
                at = o + open.len();
            }
            _ => {
                depth -= 1;
                let end = lower[c..].find('>').map(|i| c + i + 1)?;
                if depth == 0 {
                    return Some(end);
                }
                at = end;
            }
        }
    }
}

/// Every opening tag `<name ...>`.
fn tags<'a>(html: &'a str, name: &str) -> Vec<&'a str> {
    let lower = html.to_ascii_lowercase();
    let open = format!("<{name}");
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = lower[at..].find(&open).map(|i| i + at) {
        let end = html[i..].find('>').map(|e| i + e + 1).unwrap_or(html.len());
        if is_tag_start(&lower, i, name) {
            out.push(&html[i..end]);
        }
        at = end;
    }
    out
}

/// The value of attribute `name` in an opening tag, entity-decoded.
fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut at = 0;
    while let Some(i) = lower[at..].find(name).map(|i| i + at) {
        let before_ok = i > 0 && lower.as_bytes()[i - 1].is_ascii_whitespace();
        let rest = lower[i + name.len()..].trim_start();
        if before_ok && rest.starts_with('=') {
            let value_start = tag.len() - rest.len() + 1;
            let raw = tag[value_start..].trim_start();
            let value = match raw.chars().next() {
                Some(q @ ('"' | '\'')) => raw[1..].split(q).next().unwrap_or_default(),
                _ => raw.split(|c: char| c.is_whitespace() || c == '>').next().unwrap_or_default(),
            };
            return Some(decode(value));
        }
        at = i + name.len();
    }
    None
}

/// Visible text of a fragment: tags removed, entities decoded, whitespace collapsed.
pub fn text(fragment: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in fragment.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    decode(&out).split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(semi) = rest[..rest.len().min(12)].find(';') else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => u32::from_str_radix(&entity[2..], 16).ok().and_then(char::from_u32),
            _ if entity.starts_with('#') => entity[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// application/x-www-form-urlencoded, as browsers send it.
fn form_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISTRICT_PANCHAYATS: &str = include_str!("../tests/fixtures/sulekha-district-panchayats.html");

    #[test]
    fn reads_the_form_state() {
        let form = Form::parse(DISTRICT_PANCHAYATS);
        let names: Vec<&str> = form.hidden.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"__VIEWSTATE"));
        assert!(form.hidden.iter().find(|(n, _)| n == "__VIEWSTATE").is_some_and(|(_, v)| v.len() > 1000));
        let year = form.select("drpYear").expect("year drop-down");
        assert_eq!(year.current(), Some("29"));
        assert!(year.options.iter().any(|(v, label, _)| v == "29" && label.contains("2025")));
        assert_eq!(form.select("drpType").and_then(|s| s.current()), Some("1"));
    }

    #[test]
    fn a_postback_carries_the_state_and_the_click() {
        let form = Form::parse(DISTRICT_PANCHAYATS);
        let body = form.postback("gvState", "Select$6", &[("drpType", "5")]);
        assert!(body.contains("__EVENTTARGET=gvState"));
        assert!(body.contains("__EVENTARGUMENT=Select%246"));
        assert!(body.contains("drpType=5"));
        assert!(body.contains("drpYear=29"));
        assert!(body.contains("__VIEWSTATE="));
        assert!(!body.contains(' '));
    }

    #[test]
    fn reads_the_district_grid() {
        let grid = grid(DISTRICT_PANCHAYATS, "gvState").expect("gvState");
        assert_eq!(grid.headers[1], "DISTRICT");
        assert_eq!(grid.column(&["projects"]), Some(3));
        // 14 districts, then the footer with the state total.
        assert_eq!(grid.rows.len(), 15);
        assert_eq!(grid.rows[14].cells[1], "Total");
        assert!(grid.rows[14].postbacks.is_empty());
        let ernakulam = &grid.rows[6];
        assert_eq!(ernakulam.cells[1], "Ernakulam");
        assert_eq!(ernakulam.cells[3], "941");
        assert_eq!(number(&ernakulam.cells[7]), Some(12989.91));
        assert_eq!(ernakulam.postbacks, vec![("gvState".to_string(), "Select$6".to_string())]);
        assert!(grid.pages.is_empty());
    }

    #[test]
    fn pager_rows_become_page_numbers() {
        let html = r#"<table id="gvProjects"><tr><th>Sl No</th><th>Project Name</th><th>Formulation</th><th>Expense</th></tr>
            <tr><td>1</td><td>റോഡ് &amp; പാലം</td><td>5,00,000</td><td>3,21,848</td></tr>
            <tr><td colspan="4"><table><tr><td><span>1</span></td><td><a href="javascript:__doPostBack(&#39;gvProjects&#39;,&#39;Page$2&#39;)">2</a></td>
            <td><a href="javascript:__doPostBack('gvProjects','Page$3')">3</a></td></tr></table></td></tr></table>"#;
        let grid = grid(html, "gvProjects").unwrap();
        assert_eq!(grid.rows.len(), 1);
        assert_eq!(grid.rows[0].cells[1], "റോഡ് & പാലം");
        assert_eq!(grid.column(&["formulation"]), Some(2));
        assert_eq!(number(&grid.rows[0].cells[3]), Some(321848.0));
        assert_eq!(grid.pages, vec![2, 3]);
    }

    #[test]
    fn entities_and_encoding() {
        assert_eq!(decode("a &amp; b &#3330;&#x0D15; &bogus; &"), "a & b ംക &bogus; &");
        assert_eq!(form_encode("a b+/$ം"), "a+b%2B%2F%24%E0%B4%82");
        assert_eq!(number(""), None);
        assert_eq!(number("1,23,456.5"), Some(123456.5));
    }
}
