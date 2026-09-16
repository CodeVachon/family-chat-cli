//! Tiptap message-body HTML → plain text for terminal rendering.
//!
//! Message bodies are sanitized rich-text HTML (see docs/api-contract.md).
//! This collapses each top-level block element (`<p>`, `<ul>`, ...) to one
//! line of its text content — good enough to read a message; it doesn't yet
//! preserve inline formatting (bold/links/mentions) as styled spans, which is
//! a `ratatui::text::Text` upgrade for later, not a parsing problem.

pub fn to_plain_text(html: &str) -> String {
    let Ok(dom) = tl::parse(html, tl::ParserOptions::default()) else {
        return html.to_string();
    };
    let parser = dom.parser();

    let lines: Vec<String> = dom
        .children()
        .iter()
        .filter_map(|handle| handle.get(parser))
        .map(|node| decode_entities(node.inner_text(parser).trim()))
        .filter(|line| !line.is_empty())
        .collect();

    lines.join("\n")
}

/// `tl` returns text-node content verbatim, without decoding HTML entities —
/// so `&amp;` stays `&amp;` instead of becoming `&`. Sanitized rich-text
/// output only uses a handful of entities (the XML-safe escapes plus
/// `&nbsp;`), so this covers those and numeric character references rather
/// than a full named-entity table.
fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let Some(semicolon) = rest.find(';') else {
            break;
        };
        match decode_entity(&rest[1..semicolon]) {
            Some(ch) => {
                out.push(ch);
                rest = &rest[semicolon + 1..];
            }
            None => {
                // Not a recognized entity — keep the '&' literally and move
                // past just it, so an unrelated '&...;' isn't swallowed.
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        _ => {
            let digits = entity.strip_prefix('#')?;
            let code = match digits.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => digits.parse::<u32>().ok()?,
            };
            char::from_u32(code)
        }
    }
}

/// Encodes terminal-authored plain text as the sanitized HTML the API
/// expects (see docs/api-contract.md). The composer is plain text, not rich
/// text — so this escapes markup-significant characters rather than
/// interpreting them, meaning `<b>` typed in the TUI shows up literally on
/// every client instead of silently becoming bold (#58).
pub fn plain_text_to_html(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!("<p>{escaped}</p>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tags_and_joins_paragraphs() {
        let html = "<p>Hello <strong>there</strong></p><p>Second line</p>";
        assert_eq!(to_plain_text(html), "Hello there\nSecond line");
    }

    #[test]
    fn handles_plain_text_with_no_tags() {
        assert_eq!(to_plain_text("just text"), "just text");
    }

    #[test]
    fn plain_text_to_html_escapes_markup_characters() {
        assert_eq!(
            plain_text_to_html("<b>hi</b> & bye"),
            "<p>&lt;b&gt;hi&lt;/b&gt; &amp; bye</p>"
        );
    }

    #[test]
    fn round_trips_back_to_the_original_text() {
        let original = "<script>alert(1)</script> & \"quoted\"";
        let html = plain_text_to_html(original);
        assert_eq!(to_plain_text(&html), original);
    }

    #[test]
    fn decodes_numeric_and_nbsp_entities() {
        assert_eq!(
            to_plain_text("<p>caf&#233; &nbsp; &#x2764;</p>"),
            "caf\u{e9} \u{a0} \u{2764}"
        );
    }

    #[test]
    fn an_unrecognized_ampersand_use_is_left_alone() {
        assert_eq!(to_plain_text("<p>Ben & Jerry's</p>"), "Ben & Jerry's");
    }
}
