// SPDX-License-Identifier: AGPL-3.0-or-later

//! Re-indent a JSON document without parsing it into a map.
//!
//! `--format json` promises the API's body as sent. Parsing it into
//! `serde_json::Value` and printing it again would sort the keys (unless
//! serde_json's `preserve_order` feature is on, and in the app crate, where
//! the shipped `jura` is built, turning it on would change the key order of
//! every response the server writes). So this works on the text: it moves
//! whitespace and nothing else, and every byte inside a string is copied.

/// Two-space indentation, one member per line, as `serde_json`'s pretty
/// printer lays it out. The input must be valid JSON; the caller has
/// already parsed it once to check.
pub fn pretty(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() * 2);
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = raw.chars().peekable();
    let newline = |out: &mut String, depth: usize| {
        out.push('\n');
        for _ in 0..depth {
            out.push_str("  ");
        }
    };
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '{' | '[' => {
                out.push(c);
                // An empty object or array stays on one line.
                while chars.peek().is_some_and(|n| n.is_whitespace()) {
                    chars.next();
                }
                if matches!(chars.peek(), Some('}') | Some(']')) {
                    out.push(chars.next().unwrap());
                } else {
                    depth += 1;
                    newline(&mut out, depth);
                }
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
                newline(&mut out, depth);
                out.push(c);
            }
            ',' => {
                out.push(c);
                newline(&mut out, depth);
            }
            ':' => out.push_str(": "),
            c if c.is_whitespace() => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_key_order_and_string_contents() {
        let raw = r#"{"z":1,"a":{"m":[1,2,{}],"b":[]},"s":"a, b: {c} [d] \" \\","n":null}"#;
        let out = pretty(raw);
        assert_eq!(
            out,
            "{\n  \"z\": 1,\n  \"a\": {\n    \"m\": [\n      1,\n      2,\n      {}\n    ],\n    \"b\": []\n  },\n  \"s\": \"a, b: {c} [d] \\\" \\\\\",\n  \"n\": null\n}"
        );
        // The same document, by value.
        let a: serde_json::Value = serde_json::from_str(raw).unwrap();
        let b: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(a, b);
        // And the keys in the order the server sent them.
        assert!(out.find("\"z\"").unwrap() < out.find("\"a\"").unwrap());
    }

    #[test]
    fn matches_serde_json_on_ordered_input() {
        // serde_json sorts keys without preserve_order, so compare on input
        // whose keys are already sorted.
        let raw = r#"{"a":[1,{"b":true,"c":"x"}],"d":{},"e":0.55}"#;
        let v: serde_json::Value = serde_json::from_str(raw).unwrap();
        assert_eq!(pretty(raw), serde_json::to_string_pretty(&v).unwrap());
    }
}
