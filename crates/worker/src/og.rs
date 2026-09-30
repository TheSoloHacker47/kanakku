//! Share images: a 1200 x 630 card for a project, drawn on request and cached at the edge.
//!
//! Text is set in the site's own faces. Only Latin letters are drawn: Malayalam needs a text
//! shaper, and KIIFB's titles are in English anyway.

use fontdue::{Font, FontSettings};

const WIDTH: usize = 1200;
const HEIGHT: usize = 630;
const MARGIN: f32 = 64.0;

type Rgb = [u8; 3];
const PAPER: Rgb = [0xff, 0xff, 0xff];
const INK: Rgb = [0x0a, 0x0a, 0x0a];
const MUTED: Rgb = [0x5b, 0x5f, 0x5c];
const GREEN: Rgb = [0x44, 0xd9, 0x91];
const MIST: Rgb = [0xea, 0xf8, 0xf0];
const FLAG: Rgb = [0xff, 0x6a, 0x51];

/// What a card says.
pub struct Card<'a> {
    /// Small line above the title, such as a project code and district.
    pub eyebrow: &'a str,
    pub title: &'a str,
    /// The one large figure, such as "₹21.43 crore".
    pub figure: &'a str,
    /// What the figure is, such as "spent so far".
    pub figure_label: &'a str,
    /// A bar filled to this share, when the figure is a part of a whole.
    pub share: Option<f32>,
    /// Shown on the flag colour when present, such as "Overdue".
    pub flag: Option<&'a str>,
}

struct Face {
    main: Font,
    rupee: Font,
}

impl Face {
    fn load(main: &[u8], rupee: &[u8]) -> Option<Face> {
        let settings = FontSettings::default;
        Some(Face { main: Font::from_bytes(main, settings()).ok()?, rupee: Font::from_bytes(rupee, settings()).ok()? })
    }

    fn font(&self, c: char) -> &Font {
        if c == '₹' {
            &self.rupee
        } else {
            &self.main
        }
    }

    fn width(&self, text: &str, size: f32) -> f32 {
        text.chars().map(|c| self.font(c).metrics(c, size).advance_width).sum()
    }
}

struct Canvas {
    pixels: Vec<u8>,
}

impl Canvas {
    fn new(colour: Rgb) -> Canvas {
        Canvas { pixels: colour.iter().copied().cycle().take(WIDTH * HEIGHT * 3).collect() }
    }

    fn blend(&mut self, x: i32, y: i32, colour: Rgb, alpha: f32) {
        if x < 0 || y < 0 || x as usize >= WIDTH || y as usize >= HEIGHT || alpha <= 0.0 {
            return;
        }
        let at = (y as usize * WIDTH + x as usize) * 3;
        let alpha = alpha.min(1.0);
        for (i, channel) in colour.iter().enumerate() {
            let old = self.pixels[at + i] as f32;
            self.pixels[at + i] = (old + (*channel as f32 - old) * alpha).round() as u8;
        }
    }

    /// A filled rectangle with rounded corners.
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Rgb) {
        for py in y.floor() as i32..(y + h).ceil() as i32 {
            for px in x.floor() as i32..(x + w).ceil() as i32 {
                let (cx, cy) = (px as f32 + 0.5, py as f32 + 0.5);
                // Distance outside the rounded shape, for a soft edge.
                let dx = (x + radius - cx).max(cx - (x + w - radius)).max(0.0);
                let dy = (y + radius - cy).max(cy - (y + h - radius)).max(0.0);
                let outside = (dx * dx + dy * dy).sqrt() - radius;
                self.blend(px, py, colour, 0.5 - outside);
            }
        }
    }

    /// A straight stroke with round ends.
    fn stroke(&mut self, from: (f32, f32), to: (f32, f32), width: f32, colour: Rgb) {
        let half = width / 2.0;
        let (min_x, max_x) = (from.0.min(to.0) - half, from.0.max(to.0) + half);
        let (min_y, max_y) = (from.1.min(to.1) - half, from.1.max(to.1) + half);
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let length_sq = (dx * dx + dy * dy).max(1e-6);
        for py in min_y.floor() as i32..=max_y.ceil() as i32 {
            for px in min_x.floor() as i32..=max_x.ceil() as i32 {
                let (cx, cy) = (px as f32 + 0.5, py as f32 + 0.5);
                let t = (((cx - from.0) * dx + (cy - from.1) * dy) / length_sq).clamp(0.0, 1.0);
                let (nx, ny) = (from.0 + t * dx - cx, from.1 + t * dy - cy);
                self.blend(px, py, colour, 0.5 - ((nx * nx + ny * ny).sqrt() - half));
            }
        }
    }

    /// Draws one line of text with its baseline at `y`. Returns the x after the last letter.
    fn text(&mut self, face: &Face, text: &str, x: f32, y: f32, size: f32, colour: Rgb) -> f32 {
        let mut pen = x;
        for c in text.chars() {
            let (metrics, coverage) = face.font(c).rasterize(c, size);
            let left = pen.round() as i32 + metrics.xmin;
            let top = y.round() as i32 - metrics.ymin - metrics.height as i32;
            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let alpha = coverage[row * metrics.width + col] as f32 / 255.0;
                    self.blend(left + col as i32, top + row as i32, colour, alpha);
                }
            }
            pen += metrics.advance_width;
        }
        pen
    }

    fn png(&self) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(64 * 1024);
        {
            let mut encoder = png::Encoder::new(&mut out, WIDTH as u32, HEIGHT as u32);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_compression(png::Compression::Default);
            encoder.set_adaptive_filter(png::AdaptiveFilterType::Adaptive);
            let mut writer = encoder.write_header().ok()?;
            writer.write_image_data(&self.pixels).ok()?;
        }
        Some(out)
    }
}

/// Only the letters the embedded fonts carry; typographic punctuation becomes its plain form.
fn plain(text: &str) -> String {
    text.chars()
        .filter_map(|c| match c {
            '\u{2013}' | '\u{2014}' => Some('-'),
            '\u{2018}' | '\u{2019}' => Some('\''),
            '\u{201c}' | '\u{201d}' => Some('"'),
            '\u{a0}' => Some(' '),
            '₹' | '·' => Some(c),
            c if (' '..='~').contains(&c) => Some(c),
            _ => None,
        })
        .collect()
}

/// Breaks text into at most `max_lines` lines no wider than `width`, ending with "..." if cut.
fn wrap(face: &Face, text: &str, size: f32, width: f32, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = vec![String::new()];
    for word in text.split_whitespace() {
        let last = lines.len() - 1;
        let candidate = if lines[last].is_empty() { word.to_string() } else { format!("{} {word}", lines[last]) };
        if lines[last].is_empty() || face.width(&candidate, size) <= width {
            lines[last] = candidate;
        } else if lines.len() < max_lines {
            lines.push(word.to_string());
        } else {
            while !lines[last].is_empty() && face.width(&format!("{}...", lines[last]), size) > width {
                lines[last].pop();
            }
            lines[last] = format!("{}...", lines[last].trim_end());
            break;
        }
    }
    lines
}

/// Draws the card and returns it as a PNG, or `None` if a font fails to load.
pub fn render(card: &Card) -> Option<Vec<u8>> {
    let display = Face::load(include_bytes!("../fonts/display.ttf"), include_bytes!("../fonts/display-rupee.ttf"))?;
    let text = Face::load(include_bytes!("../fonts/text.ttf"), include_bytes!("../fonts/text-rupee.ttf"))?;
    let mut canvas = Canvas::new(PAPER);
    let inner = WIDTH as f32 - 2.0 * MARGIN;

    // The mark, a tally of five, and the name.
    canvas.rect(MARGIN, 52.0, 56.0, 56.0, 12.0, GREEN);
    for i in 0..4 {
        let x = MARGIN + 15.0 + i as f32 * 8.7;
        canvas.stroke((x, 66.0), (x, 94.0), 4.0, INK);
    }
    canvas.stroke((MARGIN + 9.5, 88.0), (MARGIN + 46.5, 72.0), 4.0, INK);
    canvas.text(&display, "Kanakku", MARGIN + 74.0, 96.0, 44.0, INK);

    if let Some(flag) = card.flag {
        let label = plain(flag);
        let width = text.width(&label, 26.0) + 36.0;
        canvas.rect(WIDTH as f32 - MARGIN - width, 58.0, width, 44.0, 8.0, FLAG);
        canvas.text(&text, &label, WIDTH as f32 - MARGIN - width + 18.0, 89.0, 26.0, INK);
    }

    canvas.text(&text, &plain(card.eyebrow), MARGIN, 176.0, 26.0, MUTED);
    let mut y = 232.0;
    for line in wrap(&text, &plain(card.title), 46.0, inner, 3) {
        canvas.text(&text, &line, MARGIN, y, 46.0, INK);
        y += 58.0;
    }

    // The figure sits on a band at the foot of the card.
    canvas.rect(0.0, 412.0, WIDTH as f32, 218.0, 0.0, MIST);
    let figure = plain(card.figure);
    let mut size = 104.0;
    while display.width(&figure, size) > inner && size > 48.0 {
        size -= 6.0;
    }
    canvas.text(&display, &figure, MARGIN, 522.0, size, INK);
    canvas.text(&text, &plain(card.figure_label), MARGIN, 572.0, 28.0, MUTED);
    if let Some(share) = card.share {
        canvas.rect(MARGIN, 590.0, inner, 12.0, 6.0, PAPER);
        canvas.rect(MARGIN, 590.0, (inner * share.clamp(0.0, 1.0)).max(12.0), 12.0, 6.0, GREEN);
    }
    canvas.png()
}
