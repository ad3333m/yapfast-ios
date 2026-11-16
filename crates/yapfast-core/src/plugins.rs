//! Plugins: optional behaviours in the manner of Vencord's, which Settings →
//! Plugins turns on and off. The UI sets each here, as it loads the layout and
//! on each flip; the code each one changes reads it. See docs/plugins.md.

use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plugin {
    /// Deleted messages stay, marked deleted, and an edited message shows
    /// what it said before.
    MessageLogger,
    /// Every server's emoji can be picked, animated ones too; those this
    /// account can't use go as links to their pictures.
    FakeNitro,
    /// A message that looks like it holds a card number or a password waits
    /// for a second Enter.
    SecretGuard,
    /// People's and streams' volumes go up to 400%.
    VolumeBooster,
    /// The composer's GIF and sticker buttons go.
    CleanerDiscord,
    /// The camera can't be turned on in calls.
    DisableCameras,
    /// Pinned messages say so under their text.
    PinIcon,
    /// `/google words` sends a link to Google's results for them.
    GoogleThat,
    /// A message's menu translates it into English.
    Translate,
    /// The DM button shows dots while someone types to you in a DM.
    HomeTyping,
    /// Typing in the message box clicks, as a keyboard does.
    KeyboardSounds,
}

/// Every plugin, in Settings → Plugins' order.
pub const ALL: [Plugin; 11] = [
    Plugin::MessageLogger,
    Plugin::FakeNitro,
    Plugin::SecretGuard,
    Plugin::VolumeBooster,
    Plugin::CleanerDiscord,
    Plugin::DisableCameras,
    Plugin::PinIcon,
    Plugin::GoogleThat,
    Plugin::Translate,
    Plugin::HomeTyping,
    Plugin::KeyboardSounds,
];

static ON: AtomicU32 = AtomicU32::new(0);

pub fn set(plugin: Plugin, on: bool) {
    let bit = 1 << plugin as u32;
    if on {
        ON.fetch_or(bit, Ordering::Relaxed);
    } else {
        ON.fetch_and(!bit, Ordering::Relaxed);
    }
}

pub fn on(plugin: Plugin) -> bool {
    ON.load(Ordering::Relaxed) & (1 << plugin as u32) != 0
}

/// The line MessageLogger puts under a deleted message.
const DELETED: &str = "-# (deleted)";
/// What starts each line MessageLogger puts under an edited message.
const WAS: &str = "\n-# was: ";

/// A deleted message's text as MessageLogger keeps it; `None` when it already
/// says it was deleted.
pub fn deleted(content: &str) -> Option<String> {
    if content.ends_with(DELETED) {
        return None;
    }
    Some(if content.is_empty() {
        DELETED.to_owned()
    } else {
        format!("{content}\n{DELETED}")
    })
}

/// A message's text without what MessageLogger added under it.
pub fn unlogged(content: &str) -> &str {
    let content = content.split(WAS).next().unwrap_or(content);
    content
        .strip_suffix(DELETED)
        .map_or(content, |rest| rest.strip_suffix('\n').unwrap_or(rest))
}

/// An edited message's new text with what it said before under it, after
/// what it said the times before that; `None` when the text didn't change.
pub fn edited(old: &str, new: &str) -> Option<String> {
    let before = unlogged(old);
    if before.is_empty() || before == new {
        return None;
    }
    let history = &old[before.len()..];
    let mut was: String = before.replace('\n', " ").chars().take(300).collect();
    if was.len() < before.len() {
        was.push('…');
    }
    Some(format!("{new}{history}{WAS}{was}"))
}

/// What SecretGuard sees in `text` that shouldn't be sent: a card number
/// that passes the Luhn check, or a password written out.
pub fn secret(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    for word in ["password", "passwd", "passcode"] {
        for (at, _) in lower.match_indices(word) {
            let rest = lower[at + word.len()..].trim_start();
            let value = rest
                .strip_prefix(':')
                .or_else(|| rest.strip_prefix('='))
                .or_else(|| rest.strip_prefix("is "));
            if value.is_some_and(|value| !value.trim().is_empty()) {
                return Some("a password");
            }
        }
    }
    // Runs of digits, which spaces and dashes may split.
    let mut digits = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if let Some(digit) = c.to_digit(10) {
            digits.push(digit);
        } else if !(matches!(c, ' ' | '-')
            && !digits.is_empty()
            && chars.peek().is_some_and(char::is_ascii_digit))
        {
            if card(&digits) {
                return Some("a card number");
            }
            digits.clear();
        }
    }
    card(&digits).then_some("a card number")
}

/// Amex, Visa, Mastercard and Discover numbers: 13 to 16 digits from 3 to 6,
/// which leaves out Discord's ids, 17 digits or more.
fn card(digits: &[u32]) -> bool {
    if !(13..=16).contains(&digits.len()) || !(3..=6).contains(&digits[0]) {
        return false;
    }
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(index, &digit)| match (index % 2, digit * 2) {
            (0, _) => digit,
            (_, double) if double > 9 => double - 9,
            (_, double) => double,
        })
        .sum();
    sum % 10 == 0
}

/// GoogleThat's message for `/google words`: the words, linking to Google's
/// results for them.
pub fn google(words: &str) -> String {
    let query = query_encode(words.trim());
    let words = words.trim().replace(['[', ']'], "");
    format!("[{words}](https://www.google.com/search?q={query})")
}

/// `text` as a URL query value: spaces as `+`, the rest percent-encoded
/// but for letters, digits and `-_.~`.
pub fn query_encode(text: &str) -> String {
    let mut query = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b' ' => query.push('+'),
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => query.push(byte as char),
            _ => query.push_str(&format!("%{byte:02X}")),
        }
    }
    query
}

/// Translate's request to Google's free endpoint, into English.
pub fn translate_url(text: &str) -> String {
    format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=en&dt=t&q={}",
        query_encode(text)
    )
}

/// The translation and the language it came from, from that endpoint's
/// answer: `[[["text", "original", …], …], null, "de", …]`.
pub fn translation(answer: &serde_json::Value) -> Option<(String, String)> {
    let text: String = answer[0]
        .as_array()?
        .iter()
        .filter_map(|part| part[0].as_str())
        .collect();
    let from = answer[2].as_str().unwrap_or("another language").to_owned();
    (!text.is_empty()).then_some((text, from))
}

/// A custom emoji's picture, for FakeNitro to send as a link.
/// Its text is a zero-width space, so only the picture Discord embeds shows.
pub fn emoji_link(name: &str, id: &str, animated: bool) -> String {
    let format = if animated { "gif" } else { "webp" };
    format!("[\u{200b}](https://cdn.discordapp.com/emojis/{id}.{format}?size=48&name={name}&lossless=true)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_logger_keeps_deletions_and_edits() {
        assert_eq!(deleted("hi").as_deref(), Some("hi\n-# (deleted)"));
        assert_eq!(deleted("").as_deref(), Some("-# (deleted)"));
        assert_eq!(deleted("hi\n-# (deleted)"), None);
        let once = edited("hello", "hi").unwrap();
        assert_eq!(once, "hi\n-# was: hello");
        let twice = edited(&once, "hey").unwrap();
        assert_eq!(twice, "hey\n-# was: hello\n-# was: hi");
        assert_eq!(unlogged(&twice), "hey");
        assert_eq!(unlogged("hi\n-# (deleted)"), "hi");
        assert_eq!(edited(&once, "hi"), None, "only the embeds changed");
    }

    #[test]
    fn secret_guard_finds_cards_and_passwords() {
        assert_eq!(secret("4111 1111 1111 1111"), Some("a card number"));
        assert_eq!(secret("pay 4111-1111-1111-1111 now"), Some("a card number"));
        assert_eq!(secret("4111 1111 1111 1112"), None, "fails Luhn");
        assert_eq!(secret("call 07700 900123"), None);
        assert_eq!(secret("<@1234567890123456789>"), None, "an id");
        assert_eq!(secret("my password: hunter2"), Some("a password"));
        assert_eq!(secret("I forgot my password"), None);
    }

    #[test]
    fn google_that_links_the_search() {
        assert_eq!(
            google(" rust & slint "),
            "[rust & slint](https://www.google.com/search?q=rust+%26+slint)"
        );
    }

    #[test]
    fn switches_are_apart() {
        set(Plugin::FakeNitro, true);
        assert!(on(Plugin::FakeNitro) && !on(Plugin::SecretGuard));
        set(Plugin::FakeNitro, false);
        assert!(!on(Plugin::FakeNitro));
    }
}
