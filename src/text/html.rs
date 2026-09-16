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
        .map(|node| node.inner_text(parser).trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    lines.join("\n")
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
}
