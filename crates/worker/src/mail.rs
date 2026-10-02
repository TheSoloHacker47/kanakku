//! Mail to the corrections address: forwarded to the founder, and acknowledged to the sender.

use kanakku_core::mail::{self, Incoming};
use sha2::{Digest, Sha256};
use worker::{console_error, console_log, query, Env, ForwardableEmailMessage};

/// A sender gets at most one acknowledgement a day, however many messages they send.
const ACK_EVERY_HOURS: f64 = 24.0;

pub async fn handle(message: ForwardableEmailMessage, env: &Env) {
    // Delivery comes first: an acknowledgement for a message nobody received would be a false promise.
    let Ok(target) = env.secret("FORWARD_TO").map(|s| s.to_string()) else {
        console_error!("FORWARD_TO is not set; rejecting mail so the sender knows it was not delivered");
        message.set_reject("This address is not accepting mail right now. Please try again later.");
        return;
    };
    if let Err(e) = message.forward(&target).await {
        console_error!("forwarding failed: {e:?}");
        message.set_reject("We could not deliver your message. Please try again later.");
        crate::runs::alert(env, "**Kanakku**: a message to the corrections address could not be forwarded and was bounced. Check Email Routing.").await;
        return;
    }

    let headers = message.headers();
    let header = |name: &str| headers.get(name).ok().flatten();
    let (from, to) = (message.from(), message.to());
    let (subject, message_id, references) = (header("Subject"), header("Message-ID"), header("References"));
    let (auto_submitted, precedence, list_id) = (header("Auto-Submitted"), header("Precedence"), header("List-Id"));
    let (list_unsubscribe, suppress) = (header("List-Unsubscribe"), header("X-Auto-Response-Suppress"));
    let incoming = Incoming {
        from: &from,
        to: &to,
        subject: subject.as_deref(),
        message_id: message_id.as_deref(),
        references: references.as_deref(),
        auto_submitted: auto_submitted.as_deref(),
        precedence: precedence.as_deref(),
        list_id: list_id.as_deref(),
        list_unsubscribe: list_unsubscribe.as_deref(),
        auto_response_suppress: suppress.as_deref(),
    };
    if !mail::should_ack(&incoming) {
        console_log!("forwarded; no acknowledgement (automatic or list mail)");
        return;
    }
    match first_today(env, &from).await {
        Ok(true) => {}
        Ok(false) => {
            console_log!("forwarded; sender already acknowledged today");
            return;
        }
        Err(e) => {
            console_error!("acknowledgement check failed: {e}");
            return;
        }
    }

    let now = worker::js_sys::Date::new_0();
    let date = String::from(now.to_utc_string()).replace(" GMT", " +0000");
    let id = hex(&Sha256::digest(format!("{}|{}|{from}", now.get_time(), message_id.as_deref().unwrap_or_default())))[..24].to_string();
    let raw = mail::reply(&incoming, &date, &format!("ack.{id}"));
    let sent = match worker::EmailMessage::new(&to, &from, &raw) {
        Ok(reply) => message.reply(&reply).await.map(|_| ()),
        Err(e) => Err(e),
    };
    match sent {
        Ok(()) => console_log!("forwarded and acknowledged"),
        // Usually a sender whose domain fails DMARC; Cloudflare will not reply to those.
        Err(e) => console_error!("acknowledgement not sent: {e:?}"),
    }
}

/// Records the acknowledgement and says whether this is the first to this sender in a day.
/// Only a hash of the address is kept, and only for a day.
async fn first_today(env: &Env, sender: &str) -> worker::Result<bool> {
    let db = env.d1("DB")?;
    let key = hex(&Sha256::digest(sender.trim().to_ascii_lowercase()));
    let now = crate::runs::now_iso();
    let cutoff: String = worker::js_sys::Date::new(&(worker::Date::now().as_millis() as f64 - ACK_EVERY_HOURS * 3_600_000.0).into())
        .to_iso_string()
        .into();
    #[derive(serde::Deserialize)]
    struct Row {
        #[allow(dead_code)]
        ok: u32,
    }
    let results = db
        .batch(vec![
            query!(&db, "DELETE FROM email_acks WHERE sent_at < ?1", cutoff)?,
            query!(
                &db,
                "INSERT INTO email_acks (sender_hash, sent_at) VALUES (?1, ?2)
                 ON CONFLICT (sender_hash) DO UPDATE SET sent_at = excluded.sent_at WHERE email_acks.sent_at < ?3
                 RETURNING 1 AS ok",
                key,
                now,
                cutoff
            )?,
        ])
        .await?;
    Ok(!results[1].results::<Row>()?.is_empty())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
