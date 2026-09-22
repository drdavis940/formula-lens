//! One diagnostic type, two renderings: plain text for a terminal, JSON
//! for a caller that wants to parse the result programmatically.
//!
//! This is the piece meant to back a `--json` flag on top of this
//! library: pick an [`OutputMode`] from the flag, then call
//! [`Diagnostic::render`].

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Human,
    Json,
}

impl OutputMode {
    /// Picks a mode from a boolean flag, e.g. `OutputMode::from_json_flag(args.json)`
    /// for a caller that exposes a `--json` switch on its own CLI.
    pub fn from_json_flag(json: bool) -> Self {
        if json {
            OutputMode::Json
        } else {
            OutputMode::Human
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub message: String,
    pub position: usize,
}

impl Diagnostic {
    pub fn new(message: impl Into<String>, position: usize) -> Self {
        Diagnostic { message: message.into(), position }
    }

    pub fn render(&self, mode: OutputMode) -> String {
        match mode {
            OutputMode::Human => format!("error at position {}: {}", self.position, self.message),
            OutputMode::Json => format!(
                "{{\"error\":true,\"position\":{},\"message\":\"{}\"}}",
                self.position,
                json_escape(&self.message)
            ),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render(OutputMode::Human))
    }
}

/// Minimal JSON string escaping. No dependency on a JSON crate; this
/// library only ever emits strings, arrays, objects and numbers, so it
/// doesn't need a general-purpose serializer.
pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_human_and_json() {
        let d = Diagnostic::new("bad token", 4);
        assert_eq!(d.render(OutputMode::Human), "error at position 4: bad token");
        assert_eq!(
            d.render(OutputMode::Json),
            "{\"error\":true,\"position\":4,\"message\":\"bad token\"}"
        );
    }

    #[test]
    fn escapes_quotes_and_control_chars() {
        assert_eq!(json_escape("say \"hi\"\n"), "say \\\"hi\\\"\\n");
    }
}
