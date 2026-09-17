//! Tiptap message-body HTML → styled `ratatui::text::Line`s for terminal
//! rendering (#58).
//!
//! The server's sanitizer (DOMPurify, `apps/web/lib/messaging/rich-text.ts`
//! in family-chat) allows exactly: `p br strong em s u code pre blockquote
//! ul ol li a span`, plus `href`/`target`/`rel`/`data-*`/`class` attributes,
//! with `href` restricted to `http(s):`/`mailto:`. Mentions are Tiptap's
//! default `@tiptap/extension-mention`, serialized as
//! `<span data-type="mention" data-id="..." data-label="...">@Name</span>`
//! — the text content already includes the `@`, so even the plain-text
//! fallback below shows mentions correctly without special-casing the span.
//! No `img`/`video` ever appears in body HTML; media is exclusively the
//! separate `attachments` array (see `api::types::Attachment` and
//! `tui::widgets` for how those render).

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// A message body, rendered as styled lines. Falls back to one plain line of
/// (sanitized) raw text if the HTML doesn't even parse.
pub fn to_lines(html: &str) -> Vec<Line<'static>> {
    let Ok(dom) = tl::parse(html, tl::ParserOptions::default()) else {
        return vec![Line::from(sanitize_for_terminal(&decode_entities(html)))];
    };
    let parser = dom.parser();

    let mut ctx = Ctx::default();
    for handle in dom.children() {
        if let Some(node) = handle.get(parser) {
            walk(node, parser, Fmt::default(), &mut ctx);
        }
    }
    ctx.finish_line();

    if ctx.lines.is_empty() {
        ctx.lines.push(Line::default());
    }
    ctx.lines
}

#[derive(Debug, Clone, Copy, Default)]
struct Fmt {
    style: Style,
    /// Nesting depth inside `<blockquote>` — each finished line inside one
    /// gets a `"> "` per level of leading quote marker.
    quote_depth: usize,
}

#[derive(Default)]
struct Ctx {
    lines: Vec<Line<'static>>,
    current: Vec<Span<'static>>,
}

impl Ctx {
    fn push_text(&mut self, text: &str, style: Style) {
        let clean = sanitize_for_terminal(text);
        if clean.is_empty() {
            return;
        }
        self.current.push(Span::styled(clean, style));
    }

    /// Ends the current line, prefixing it with `"> "` per blockquote level
    /// it was written under. A no-op (produces no empty line) when nothing
    /// has been written since the last break.
    fn finish_line(&mut self) {
        self.finish_line_quoted(0);
    }

    fn finish_line_quoted(&mut self, quote_depth: usize) {
        if self.current.is_empty() {
            return;
        }
        let mut spans = std::mem::take(&mut self.current);
        if quote_depth > 0 {
            spans.insert(0, Span::raw("> ".repeat(quote_depth)));
        }
        self.lines.push(Line::from(spans));
    }
}

fn walk(node: &tl::Node, parser: &tl::Parser, fmt: Fmt, ctx: &mut Ctx) {
    match node {
        tl::Node::Comment(_) => {}
        tl::Node::Raw(bytes) => {
            ctx.push_text(&decode_entities(&bytes.as_utf8_str()), fmt.style);
        }
        tl::Node::Tag(tag) => walk_tag(tag, parser, fmt, ctx),
    }
}

fn walk_children(tag: &tl::HTMLTag, parser: &tl::Parser, fmt: Fmt, ctx: &mut Ctx) {
    for handle in tag.children().top().iter() {
        if let Some(node) = handle.get(parser) {
            walk(node, parser, fmt, ctx);
        }
    }
}

/// Ends the current line (respecting `fmt`'s quote depth) — for block-level
/// elements, called both before and after their content so they never share
/// a line with a preceding or following sibling.
fn break_block(fmt: Fmt, ctx: &mut Ctx) {
    ctx.finish_line_quoted(fmt.quote_depth);
}

fn walk_tag(tag: &tl::HTMLTag, parser: &tl::Parser, fmt: Fmt, ctx: &mut Ctx) {
    match tag.name().as_utf8_str().as_ref() {
        "br" => break_block(fmt, ctx),

        "p" | "div" | "pre" | "blockquote" => {
            break_block(fmt, ctx);
            let inner = if tag.name().as_utf8_str() == "pre" {
                Fmt {
                    style: fmt.style.fg(Color::Cyan),
                    ..fmt
                }
            } else if tag.name().as_utf8_str() == "blockquote" {
                Fmt {
                    style: fmt.style.fg(Color::Gray).add_modifier(Modifier::ITALIC),
                    quote_depth: fmt.quote_depth + 1,
                }
            } else {
                fmt
            };
            walk_children(tag, parser, inner, ctx);
            break_block(inner, ctx);
        }

        "ul" => {
            break_block(fmt, ctx);
            for handle in tag.children().top().iter() {
                let Some(tl::Node::Tag(li)) = handle.get(parser) else {
                    continue;
                };
                ctx.push_text("• ", fmt.style);
                walk_children(li, parser, fmt, ctx);
                break_block(fmt, ctx);
            }
        }
        "ol" => {
            break_block(fmt, ctx);
            for (i, handle) in tag.children().top().iter().enumerate() {
                let Some(tl::Node::Tag(li)) = handle.get(parser) else {
                    continue;
                };
                ctx.push_text(&format!("{}. ", i + 1), fmt.style);
                walk_children(li, parser, fmt, ctx);
                break_block(fmt, ctx);
            }
        }
        // A bare `<li>` outside `ul`/`ol` (malformed input) — still break it
        // out onto its own line rather than running it into its neighbors.
        "li" => {
            break_block(fmt, ctx);
            walk_children(tag, parser, fmt, ctx);
            break_block(fmt, ctx);
        }

        "strong" | "b" => walk_children(tag, parser, bold(fmt), ctx),
        "em" | "i" => walk_children(tag, parser, italic(fmt), ctx),
        "u" => walk_children(tag, parser, underline(fmt), ctx),
        "s" | "strike" | "del" => walk_children(tag, parser, strikethrough(fmt), ctx),
        "code" => walk_children(
            tag,
            parser,
            Fmt {
                style: fmt.style.fg(Color::Cyan),
                ..fmt
            },
            ctx,
        ),

        "a" => {
            let href = attr(tag, "href");
            let link_fmt = Fmt {
                style: fmt.style.fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
                ..fmt
            };
            walk_children(tag, parser, link_fmt, ctx);
            if let Some(href) = href {
                ctx.push_text(&format!(" ({href})"), fmt.style.fg(Color::DarkGray));
            }
        }

        "span" if attr(tag, "data-type").as_deref() == Some("mention") => {
            walk_children(
                tag,
                parser,
                Fmt {
                    style: fmt.style.fg(Color::Magenta).add_modifier(Modifier::BOLD),
                    ..fmt
                },
                ctx,
            );
        }

        // Unrecognized tag (or a plain `<span>`): pass through as inline
        // content in the current style rather than dropping it — matches
        // "preserve readable fallback text for unsupported ... markup".
        _ => walk_children(tag, parser, fmt, ctx),
    }
}

fn bold(fmt: Fmt) -> Fmt {
    Fmt {
        style: fmt.style.add_modifier(Modifier::BOLD),
        ..fmt
    }
}
fn italic(fmt: Fmt) -> Fmt {
    Fmt {
        style: fmt.style.add_modifier(Modifier::ITALIC),
        ..fmt
    }
}
fn underline(fmt: Fmt) -> Fmt {
    Fmt {
        style: fmt.style.add_modifier(Modifier::UNDERLINED),
        ..fmt
    }
}
fn strikethrough(fmt: Fmt) -> Fmt {
    Fmt {
        style: fmt.style.add_modifier(Modifier::CROSSED_OUT),
        ..fmt
    }
}

fn attr(tag: &tl::HTMLTag, name: &str) -> Option<String> {
    let value = tag.attributes().get(name)??;
    Some(decode_entities(&value.as_utf8_str()))
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

/// Drops ASCII/C1 control characters other than `\n`/`\t` (harmless as a
/// single cell each; `\n` inside a text node is a rare, legitimate literal
/// line break, not something we split on ourselves the way `<br>` is).
///
/// Message bodies pass through the server's HTML sanitizer, but that only
/// strips dangerous *tags/attributes* — a literal ESC byte sitting in a text
/// node is perfectly valid sanitized HTML and would reach a naive renderer
/// unchanged. Ratatui writes span content to the terminal as raw bytes, so
/// an unstripped ESC here would let a message author inject real terminal
/// escape sequences (cursor moves, OSC title-bar/clipboard tricks, etc.) —
/// exactly what #58's plan means by "never interpret terminal escape
/// sequences from content." This is the one thing standing between that
/// requirement and reality, so every text run must go through it before
/// becoming a `Span`.
fn sanitize_for_terminal(input: &str) -> String {
    input
        .chars()
        .filter(|&c| c == '\n' || c == '\t' || !c.is_control())
        .collect()
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

    /// Flattens rendered lines back to plain text (ignoring style) so most
    /// tests can assert on content without hand-writing `Line`/`Span`
    /// literals.
    fn plain(html: &str) -> String {
        to_lines(html)
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn strips_tags_and_joins_paragraphs() {
        let html = "<p>Hello <strong>there</strong></p><p>Second line</p>";
        assert_eq!(plain(html), "Hello there\nSecond line");
    }

    #[test]
    fn handles_plain_text_with_no_tags() {
        assert_eq!(plain("just text"), "just text");
    }

    #[test]
    fn br_is_a_line_break_within_a_paragraph() {
        // A real gap in the old plain-text renderer: a soft break (Shift+Enter
        // on the web) produced no separator at all, running two lines together.
        assert_eq!(plain("<p>one<br>two</p>"), "one\ntwo");
    }

    #[test]
    fn bold_text_gets_the_bold_modifier() {
        let lines = to_lines("<p>plain <strong>bold</strong></p>");
        let bold_span = &lines[0].spans[1];
        assert_eq!(bold_span.content.as_ref(), "bold");
        assert!(bold_span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn a_link_shows_its_own_text_and_the_href() {
        assert_eq!(
            plain(r#"<p>see <a href="https://example.com">here</a></p>"#),
            "see here (https://example.com)"
        );
    }

    #[test]
    fn a_mention_span_keeps_its_at_prefixed_text() {
        assert_eq!(
            plain(
                r#"<p>hi <span data-type="mention" data-id="u1" data-label="Chris">@Chris</span></p>"#
            ),
            "hi @Chris"
        );
    }

    #[test]
    fn unordered_list_items_are_bulleted_on_their_own_lines() {
        assert_eq!(plain("<ul><li>a</li><li>b</li></ul>"), "• a\n• b");
    }

    #[test]
    fn ordered_list_items_are_numbered() {
        assert_eq!(plain("<ol><li>a</li><li>b</li></ol>"), "1. a\n2. b");
    }

    #[test]
    fn blockquote_lines_get_a_quote_marker() {
        assert_eq!(plain("<blockquote><p>quoted</p></blockquote>"), "> quoted");
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
        assert_eq!(plain(&html), original);
    }

    #[test]
    fn decodes_numeric_and_nbsp_entities() {
        assert_eq!(
            plain("<p>caf&#233; &nbsp; &#x2764;</p>"),
            "caf\u{e9} \u{a0} \u{2764}"
        );
    }

    #[test]
    fn an_unrecognized_ampersand_use_is_left_alone() {
        assert_eq!(plain("<p>Ben & Jerry's</p>"), "Ben & Jerry's");
    }

    #[test]
    fn control_characters_never_reach_a_span() {
        // A literal ESC (start of a terminal escape sequence) embedded in a
        // text node: valid sanitized HTML (the sanitizer only strips
        // tags/attributes, not control bytes in text), but must never reach
        // the terminal raw — see `sanitize_for_terminal`'s doc comment.
        let malicious = "<p>hello\u{1b}[31mRED\u{1b}[0m\u{7}world</p>";
        let rendered = plain(malicious);
        assert!(
            !rendered.contains('\u{1b}'),
            "ESC leaked through: {rendered:?}"
        );
        assert!(
            !rendered.contains('\u{7}'),
            "BEL leaked through: {rendered:?}"
        );
        // The ESC/BEL *bytes* are gone (the actual security property — a
        // terminal can't interpret an escape sequence with no ESC in it),
        // but the printable characters that happened to follow them are
        // ordinary text and are kept, same as any other message content.
        assert_eq!(rendered, "hello[31mRED[0mworld");
    }

    #[test]
    fn malformed_html_falls_back_to_sanitized_raw_text() {
        // tl's parser is lenient and rarely hard-fails, but the fallback
        // path (unparseable input) must still not leak control characters.
        let rendered = plain("plain \u{1b} text");
        assert!(!rendered.contains('\u{1b}'));
    }
}
