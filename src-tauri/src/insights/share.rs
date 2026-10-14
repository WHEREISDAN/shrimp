//! Sharing the stats card to X or LinkedIn (STA-03): the card goes on the
//! clipboard to paste into the post, and the network's compose page opens
//! with a line of text.

use serde::Deserialize;

use crate::tn;

/// Mirrors `ShareNetwork` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShareNetwork {
    X,
    LinkedIn,
}

/// X's documented web intent (docs.x.com, "Web Intent"); `text` pre-fills the post.
const X_COMPOSE: &str = "https://x.com/intent/tweet?text=";
/// LinkedIn documents no compose URL that takes text (its share-offsite URL
/// only shares a link); this is the feed's composer, as Wispr Flow opens it.
const LINKEDIN_COMPOSE: &str = "https://www.linkedin.com/feed/?shareActive=true&text=";

/// "I've dictated 12,345 words with Shrimp." in the interface language.
pub fn post_text(total_words: i64) -> String {
    tn!("share.post", total_words)
}

/// The compose page of `network` with `text` filled in.
pub fn compose_url(network: ShareNetwork, text: &str) -> String {
    let base = match network {
        ShareNetwork::X => X_COMPOSE,
        ShareNetwork::LinkedIn => LINKEDIN_COMPOSE,
    };
    format!("{base}{}", encode(text))
}

/// Percent-encodes everything but RFC 3986's unreserved characters, as
/// UTF-8 (a space is `%20`).
fn encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn networks_deserialize_from_camel_case() {
        let x: ShareNetwork = serde_json::from_str("\"x\"").unwrap();
        let linkedin: ShareNetwork = serde_json::from_str("\"linkedIn\"").unwrap();
        assert_eq!((x, linkedin), (ShareNetwork::X, ShareNetwork::LinkedIn));
    }

    #[test]
    fn compose_pages_carry_the_encoded_text() {
        let text = post_text(12_345);
        assert_eq!(text, "I've dictated 12,345 words with Shrimp.");
        assert_eq!(post_text(1), "I've dictated 1 word with Shrimp.");
        assert_eq!(
            compose_url(ShareNetwork::X, &text),
            "https://x.com/intent/tweet?text=I%27ve%20dictated%2012%2C345%20words%20with%20Shrimp."
        );
        assert_eq!(
            compose_url(ShareNetwork::LinkedIn, "Grüße & co"),
            "https://www.linkedin.com/feed/?shareActive=true&text=Gr%C3%BC%C3%9Fe%20%26%20co"
        );
    }
}
