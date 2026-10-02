//! The automatic acknowledgement sent to anyone who writes to the corrections address.
//!
//! The reply promises what the About page promises: an answer within seven days, and a disputed
//! figure marked as under review. It never goes to robots, mailing lists or other auto-replies,
//! so two autoresponders cannot answer each other forever.

/// Header values of the incoming message that decide whether it gets a reply. Missing headers are `None`.
#[derive(Debug, Default, Clone)]
pub struct Incoming<'a> {
    /// The envelope sender (MAIL FROM). Empty for bounces.
    pub from: &'a str,
    /// The address that received the message, which the reply is sent from.
    pub to: &'a str,
    pub subject: Option<&'a str>,
    pub message_id: Option<&'a str>,
    pub references: Option<&'a str>,
    pub auto_submitted: Option<&'a str>,
    pub precedence: Option<&'a str>,
    pub list_id: Option<&'a str>,
    pub list_unsubscribe: Option<&'a str>,
    pub auto_response_suppress: Option<&'a str>,
}

/// Mailbox names that belong to machines, not people.
const ROBOT_MAILBOXES: [&str; 8] = ["mailer-daemon", "postmaster", "noreply", "no-reply", "donotreply", "do-not-reply", "bounce", "notification"];

/// Cloudflare refuses to reply when `References` has more than 100 entries; stop well before that.
const MAX_REFERENCES: usize = 50;

/// Whether the message should get the acknowledgement.
pub fn should_ack(m: &Incoming) -> bool {
    let from = m.from.trim().trim_matches(|c| c == '<' || c == '>').to_ascii_lowercase();
    let Some((mailbox, domain)) = from.rsplit_once('@') else { return false };
    if mailbox.is_empty() || domain.is_empty() || from == m.to.trim().to_ascii_lowercase() {
        return false;
    }
    if ROBOT_MAILBOXES.iter().any(|robot| mailbox.contains(robot)) {
        return false;
    }
    // RFC 3834: anything automatic says so, and only "no" means a person sent it.
    if m.auto_submitted.is_some_and(|v| !v.trim().eq_ignore_ascii_case("no")) {
        return false;
    }
    if m.precedence.is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "bulk" | "list" | "junk" | "auto_reply")) {
        return false;
    }
    if m.list_id.is_some() || m.list_unsubscribe.is_some() {
        return false;
    }
    if m.auto_response_suppress.is_some_and(|v| {
        let v = v.to_ascii_lowercase();
        v.contains("all") || v.contains("autoreply") || v.contains("oof")
    }) {
        return false;
    }
    m.references.map_or(0, |r| r.split_whitespace().count()) < MAX_REFERENCES
}

/// The acknowledgement in Malayalam, then English.
pub const BODY: &str = "\
നമസ്കാരം,

കണക്കിലേക്ക് എഴുതിയതിന് നന്ദി. നിങ്ങളുടെ സന്ദേശം ഞങ്ങൾക്ക് ലഭിച്ചു. ഏഴു ദിവസത്തിനകം മറുപടി നൽകാനാണ് ഞങ്ങൾ ശ്രമിക്കുന്നത്. ഒരു കണക്കോ ഫ്ലാഗോ തെറ്റാണെന്നാണ് നിങ്ങൾ അറിയിക്കുന്നതെങ്കിൽ, ഉറവിടവുമായി ഒത്തുനോക്കുന്നതുവരെ അത് \"പരിശോധനയിൽ\" എന്ന് അടയാളപ്പെടുത്തും.

നിങ്ങളുടെ ഇമെയിൽ വിലാസം മറുപടി നൽകാൻ മാത്രമേ ഉപയോഗിക്കൂ; മറ്റാർക്കും കൈമാറില്ല.

ഇത് സ്വയം അയക്കുന്ന മറുപടിയാണ്. ഇതിന് മറുപടി നൽകേണ്ടതില്ല.

---

Hello,

Thank you for writing to Kanakku. Your message has reached us. We aim to reply within seven days. If you are telling us that a figure or flag is wrong, we mark it as under review while we check it against the source.

We use your address only to reply to you, and we do not share it.

This is an automatic reply; there is no need to answer it.

Kanakku
https://keralakanakku.com/en/about
";

/// The reply as a raw MIME message. `date` is an RFC 5322 date and `id` a unique token for the Message-ID.
pub fn reply(m: &Incoming, date: &str, id: &str) -> String {
    let own = m.to.trim().trim_matches(|c| c == '<' || c == '>');
    let sender = m.from.trim().trim_matches(|c| c == '<' || c == '>');
    let domain = own.rsplit_once('@').map_or("localhost", |(_, d)| d);

    let subject = one_line(m.subject.unwrap_or_default());
    let subject = if subject.is_empty() {
        "Re: your message to Kanakku".to_string()
    } else if subject.get(..3).is_some_and(|start| start.eq_ignore_ascii_case("re:")) {
        subject
    } else {
        format!("Re: {subject}")
    };

    let mut out = String::new();
    let mut header = |name: &str, value: &str| {
        out.push_str(name);
        out.push_str(": ");
        out.push_str(value);
        out.push_str("\r\n");
    };
    header("From", &format!("Kanakku <{own}>"));
    header("To", &format!("<{sender}>"));
    header("Subject", &encode_header(&subject));
    header("Date", date);
    header("Message-ID", &format!("<{id}@{domain}>"));
    if let Some(parent) = m.message_id.map(one_line).filter(|s| !s.is_empty()) {
        header("In-Reply-To", &parent);
        let references = m.references.map(one_line).filter(|s| !s.is_empty());
        header("References", &references.map_or(parent.clone(), |r| format!("{r} {parent}")));
    }
    header("Auto-Submitted", "auto-replied");
    header("X-Auto-Response-Suppress", "All");
    header("MIME-Version", "1.0");
    header("Content-Type", "text/plain; charset=utf-8");
    header("Content-Transfer-Encoding", "base64");
    out.push_str("\r\n");
    // Text in mail is CRLF-delimited, encoded or not.
    let encoded = base64(BODY.replace('\n', "\r\n").as_bytes());
    for line in encoded.as_bytes().chunks(76) {
        out.push_str(std::str::from_utf8(line).unwrap_or_default());
        out.push_str("\r\n");
    }
    out
}

/// Header values arrive unfolded but may still carry line breaks; a break would end the header early.
fn one_line(s: &str) -> String {
    s.split(['\r', '\n']).map(str::trim).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" ")
}

/// ASCII passes through (including subjects that are already encoded words); anything else
/// becomes a single RFC 2047 encoded word.
fn encode_header(value: &str) -> String {
    if value.is_ascii() {
        value.to_string()
    } else {
        format!("=?UTF-8?B?{}?=", base64(value.as_bytes()))
    }
}

pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> shift & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person() -> Incoming<'static> {
        Incoming {
            from: "reader@example.org",
            to: "corrections@keralakanakku.com",
            subject: Some("Wrong date on PWD015-15"),
            message_id: Some("<abc@example.org>"),
            ..Incoming::default()
        }
    }

    #[test]
    fn a_person_gets_a_reply() {
        assert!(should_ack(&person()));
        assert!(should_ack(&Incoming { auto_submitted: Some("no"), ..person() }));
    }

    #[test]
    fn machines_and_lists_do_not() {
        for m in [
            Incoming { from: "", ..person() },
            Incoming { from: "<>", ..person() },
            Incoming { from: "MAILER-DAEMON@mx.example.org", ..person() },
            Incoming { from: "no-reply@accounts.example.com", ..person() },
            Incoming { from: "bounces+123@mail.example.com", ..person() },
            Incoming { from: "corrections@keralakanakku.com", ..person() },
            Incoming { auto_submitted: Some("auto-replied"), ..person() },
            Incoming { precedence: Some("bulk"), ..person() },
            Incoming { list_id: Some("<datameet.googlegroups.com>"), ..person() },
            Incoming { list_unsubscribe: Some("<mailto:x@example.org>"), ..person() },
            Incoming { auto_response_suppress: Some("OOF, AutoReply"), ..person() },
        ] {
            assert!(!should_ack(&m), "{m:?}");
        }
        let long = "<a@b> ".repeat(MAX_REFERENCES);
        assert!(!should_ack(&Incoming { references: Some(&long), ..person() }));
    }

    #[test]
    fn the_reply_threads_and_marks_itself_automatic() {
        let raw = reply(&Incoming { references: Some("<root@example.org>"), ..person() }, "Fri, 02 Oct 2026 10:00:00 +0000", "k1");
        let (head, body) = raw.split_once("\r\n\r\n").unwrap();
        assert!(head.contains("From: Kanakku <corrections@keralakanakku.com>\r\n"));
        assert!(head.contains("To: <reader@example.org>\r\n"));
        assert!(head.contains("Subject: Re: Wrong date on PWD015-15\r\n"));
        assert!(head.contains("Message-ID: <k1@keralakanakku.com>\r\n"));
        assert!(head.contains("In-Reply-To: <abc@example.org>\r\n"));
        assert!(head.contains("References: <root@example.org> <abc@example.org>\r\n"));
        assert!(head.contains("Auto-Submitted: auto-replied\r\n"));
        assert!(body.lines().all(|l| l.len() <= 76));
        assert_eq!(body.replace("\r\n", ""), base64(BODY.replace('\n', "\r\n").as_bytes()));
    }

    #[test]
    fn subjects_are_kept_on_one_line_and_encoded() {
        let raw = reply(&Incoming { subject: Some("Re: hello\r\nBcc: x@evil.example"), ..person() }, "d", "k");
        assert!(raw.contains("Subject: Re: hello Bcc: x@evil.example\r\n"));
        let raw = reply(&Incoming { subject: Some("പാലം"), message_id: None, ..person() }, "d", "k");
        assert!(raw.contains(&format!("Subject: =?UTF-8?B?{}?=\r\n", base64("Re: പാലം".as_bytes()))));
        assert!(!raw.contains("In-Reply-To"));
        let raw = reply(&Incoming { subject: None, ..person() }, "d", "k");
        assert!(raw.contains("Subject: Re: your message to Kanakku\r\n"));
        // A multi-byte character across byte 3 must not panic.
        assert!(reply(&Incoming { subject: Some("aപാലം"), ..person() }, "d", "k").contains("Subject: =?UTF-8?B?"));
    }

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
