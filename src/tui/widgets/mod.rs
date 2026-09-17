//! Channel list, message pane, composer, and auth-status widgets (#24/#26).

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::api::types::{Attachment, Message, MessageAuthor};
use crate::app::AppState;
use crate::app::state::{LoggedInFocus, LoggedInState, LoginField, LoginForm, Screen};
use crate::text::html;

use super::layout::split;

pub fn render(frame: &mut Frame, state: &AppState) {
    match &state.screen {
        Screen::Resuming => centered_message(frame, "Connecting…"),
        Screen::LoggedOut(form) => login_form(frame, form),
        Screen::LoggingIn => centered_message(frame, "Signing in…"),
        Screen::NotApproved => centered_message(
            frame,
            "Signed in, but this account isn't approved yet.\n\
             An admin needs to approve you before you can continue.\n\n\
             Press q to quit.",
        ),
        Screen::LoggedIn(logged_in) => logged_in_view(frame, logged_in),
    }
}

fn centered_message(frame: &mut Frame, message: &str) {
    let area = frame.area();
    let paragraph = Paragraph::new(message)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("family-chat"));
    frame.render_widget(paragraph, centered_rect(60, 30, area));
}

fn centered_rect(width_pct: u16, height_pct: u16, area: Rect) -> Rect {
    let [_, vertical, _] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_pct) / 2),
            Constraint::Percentage(height_pct),
            Constraint::Percentage((100 - height_pct) / 2),
        ])
        .areas(area);
    let [_, horizontal, _] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_pct) / 2),
            Constraint::Percentage(width_pct),
            Constraint::Percentage((100 - width_pct) / 2),
        ])
        .areas(vertical);
    horizontal
}

fn login_form(frame: &mut Frame, form: &LoginForm) {
    let area = centered_rect(50, 40, frame.area());
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title("Sign in — family chat"),
        area,
    );
    let inner = inner_area(area);

    let [email_area, password_area, error_area, help_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .areas(inner);

    render_field(
        frame,
        email_area,
        "Email",
        &form.email,
        form.focus == Some(LoginField::Email),
        false,
    );
    render_field(
        frame,
        password_area,
        "Password",
        &form.password,
        form.focus == Some(LoginField::Password),
        true,
    );

    let (focused_area, focused_len) = match form.focus.unwrap_or(LoginField::Email) {
        LoginField::Email => (email_area, form.email.chars().count()),
        LoginField::Password => (password_area, form.password.chars().count()),
    };
    let cursor_x = (focused_area.x + 1 + focused_len as u16)
        .min(focused_area.x + focused_area.width.saturating_sub(2));
    frame.set_cursor_position((cursor_x, focused_area.y + 1));

    if let Some(error) = &form.error {
        frame.render_widget(
            Paragraph::new(error.as_str()).style(Style::default().fg(Color::Red)),
            error_area,
        );
    }

    frame.render_widget(
        Paragraph::new("Tab to switch fields · Enter to sign in · Ctrl-C to quit"),
        help_area,
    );
}

fn render_field(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: &str,
    focused: bool,
    mask: bool,
) {
    let display = if mask {
        "•".repeat(value.chars().count())
    } else {
        value.to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(focus_style(focused));
    frame.render_widget(Paragraph::new(display).block(block), area);
}

fn inner_area(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

fn logged_in_view(frame: &mut Frame, state: &LoggedInState) {
    let layout = split(frame.area());
    let [messages_area, compose_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .areas(layout.main);

    let channels = state.channels.as_deref().unwrap_or(&[]);
    let items: Vec<ListItem> = channels
        .iter()
        .enumerate()
        .map(|(i, channel)| {
            let mut label = format!("# {}", channel.name);
            if channel.unread_count > 0 {
                label.push_str(&format!(" ({})", channel.unread_count));
            }
            let style = if i == state.selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(label).style(style)
        })
        .collect();
    let sidebar_style = focus_style(state.focus == LoggedInFocus::Channels);
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Channels")
                .style(sidebar_style),
        ),
        layout.sidebar,
    );

    let title = state
        .selected_channel()
        .map(|c| format!("# {}", c.name))
        .unwrap_or_else(|| "family-chat".to_string());
    let body: Vec<Line> = if channels.is_empty() {
        vec![Line::from("No channels yet.")]
    } else if state.loading_messages {
        vec![Line::from("Loading messages…")]
    } else {
        match state
            .selected_channel()
            .and_then(|c| state.messages.get(&c.id))
        {
            Some(messages) if messages.is_empty() => vec![Line::from("No messages yet.")],
            Some(messages) => {
                let all_lines: Vec<Line> = messages.iter().flat_map(message_lines).collect();
                // Window to the pane's visible height, anchored to the
                // bottom (latest) minus however far `message_scroll` has
                // paged up — otherwise a channel with more history than
                // fits would just clip the newest messages off the bottom
                // with no way to see them. Counts raw lines rather than
                // post-wrap rendered rows, so a single very long line can
                // still push things off by a row or two; an acceptable
                // approximation for how short most chat messages are.
                windowed(all_lines, visible_rows(messages_area), state.message_scroll)
            }
            None => vec![],
        }
    };
    frame.render_widget(
        Paragraph::new(Text::from(body))
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: false }),
        messages_area,
    );

    let compose_focused = state.focus == LoggedInFocus::Compose;
    let compose_title = if state.sending {
        "Sending…"
    } else {
        "Message"
    };
    frame.render_widget(
        Paragraph::new(state.compose.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(compose_title)
                .style(focus_style(compose_focused)),
        ),
        compose_area,
    );
    if compose_focused {
        // Puts the terminal's real cursor at the end of the draft so typing
        // feels like a normal text input rather than a static display.
        let cursor_x = compose_area.x + 1 + state.compose.chars().count() as u16;
        let cursor_x = cursor_x.min(compose_area.x + compose_area.width.saturating_sub(2));
        frame.set_cursor_position((cursor_x, compose_area.y + 1));
    }

    let status = state.status.clone().unwrap_or_else(|| {
        format!(
            "Signed in as {} · Tab switch focus · \u{2191}/\u{2193} channels · PgUp/PgDn scroll · Enter send · r refresh · l logout · q quit",
            state.user.name
        )
    });
    frame.render_widget(Paragraph::new(status), layout.status);
}

/// A bordered pane's inner content height.
fn visible_rows(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize
}

/// The last `height` lines, offset upward by `scroll` lines — i.e. what a
/// bottom-anchored, scroll-up-for-history pane shows. `scroll` is clamped so
/// scrolling past the start shows the first page rather than going blank
/// (state::AppState doesn't know the line count when it tracks
/// `message_scroll`, so it can't clamp on that side).
fn windowed<'a>(lines: Vec<Line<'a>>, height: usize, scroll: usize) -> Vec<Line<'a>> {
    let scroll = scroll.min(lines.len().saturating_sub(height));
    let end = lines.len() - scroll;
    let start = end.saturating_sub(height);
    lines[start..end].to_vec()
}

/// One message: an author/timestamp-prefixed rendering of its (rich-text)
/// body, followed by one line per attachment.
fn message_lines(message: &Message) -> Vec<Line<'static>> {
    let prefix = vec![
        Span::styled(
            format!("[{}] ", message.created_at.format("%H:%M")),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("{}: ", message.author.display_name()),
            Style::default()
                .fg(author_color(&message.author))
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let mut lines = html::to_lines(&message.body);
    match lines.first_mut() {
        Some(first) => {
            let mut spans = prefix;
            spans.append(&mut first.spans);
            *first = Line::from(spans);
        }
        None => lines.push(Line::from(prefix)),
    }

    lines.extend(message.attachments.iter().map(attachment_line));
    lines
}

fn attachment_line(attachment: &Attachment) -> Line<'static> {
    let label = match attachment.kind.as_str() {
        "image" => "image",
        "video" => "video",
        "pdf" => "pdf",
        _ => "file",
    };
    let name = attachment
        .original_filename
        .clone()
        .unwrap_or_else(|| "attachment".to_string());
    let dimensions = match (attachment.width, attachment.height) {
        (Some(w), Some(h)) => format!(" ({w}x{h})"),
        _ => String::new(),
    };
    // No inline image/video rendering — a terminal has no reliable universal
    // way to do that (sixel/kitty graphics support varies, and video can
    // never render inline at all). The raw URL is plain visible text
    // instead of an OSC-8 hyperlink: several modern terminals auto-linkify
    // bare URLs on their own, and embedding raw escape sequences directly in
    // a ratatui cell isn't safe (see `text::html`'s control-character note).
    Line::from(vec![
        Span::styled(format!("[{label}] "), Style::default().fg(Color::Green)),
        Span::raw(format!("{name}{dimensions} — ")),
        Span::styled(
            attachment.secure_url.clone(),
            Style::default().fg(Color::DarkGray),
        ),
    ])
}

/// A stable-per-author color so messages in a busy channel are easier to
/// tell apart at a glance. Hashes the author id (not the display name, which
/// can change) into a small, readable palette.
fn author_color(author: &MessageAuthor) -> Color {
    const PALETTE: [Color; 6] = [
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::LightRed,
    ];
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    author.id.hash(&mut hasher);
    PALETTE[(hasher.finish() as usize) % PALETTE.len()]
}

fn focus_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(labels: &[&str]) -> Vec<Line<'static>> {
        labels
            .iter()
            .map(|s| Line::from((*s).to_string()))
            .collect()
    }

    fn labels(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }

    #[test]
    fn shows_the_tail_when_content_overflows_the_pane() {
        assert_eq!(
            labels(&windowed(lines(&["1", "2", "3", "4", "5"]), 3, 0)),
            ["3", "4", "5"]
        );
    }

    #[test]
    fn scrolling_up_shows_older_lines() {
        assert_eq!(
            labels(&windowed(lines(&["1", "2", "3", "4", "5"]), 3, 2)),
            ["1", "2", "3"]
        );
    }

    #[test]
    fn scrolling_past_the_start_clamps_rather_than_panics() {
        assert_eq!(
            labels(&windowed(lines(&["1", "2", "3"]), 3, 100)),
            ["1", "2", "3"]
        );
    }

    #[test]
    fn short_content_is_shown_in_full() {
        assert_eq!(labels(&windowed(lines(&["1", "2"]), 10, 0)), ["1", "2"]);
    }

    // Full-render checks against a synthetic (never real-account) fixture —
    // this is what actually caught that overflowing history was clipping
    // the newest messages off the bottom with no way to scroll to them.

    use chrono::Utc;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::api::types::{Channel, Message, MessageAuthor, User};
    use crate::app::{AppState, Command};

    fn logged_in_state_with_messages(count: usize) -> AppState {
        let mut state = AppState::resuming();
        state.on_resume_finished(Some(User {
            id: "u1".into(),
            name: "Chris".into(),
            email: "chris@example.com".into(),
            approval_status: "approved".into(),
        }));
        let channels = vec![Channel {
            id: "c1".into(),
            name: "General".into(),
            description: None,
            is_private: false,
            is_archived: false,
            is_favorite: false,
            unread_count: 0,
        }];
        let Some(Command::LoadMessages { seq, .. }) = state.on_channels_loaded(Ok(channels)) else {
            panic!("expected the initial channel load to request messages");
        };
        let messages = (0..count)
            .map(|i| Message {
                id: format!("m{i}"),
                kind: "user".to_string(),
                body: format!("<p>msg-{i}</p>"),
                created_at: Utc::now(),
                deleted_at: None,
                author: MessageAuthor {
                    id: "u1".to_string(),
                    name: "Chris".to_string(),
                    preferences: None,
                },
                attachments: Vec::new(),
            })
            .collect();
        state.on_messages_loaded("c1".to_string(), seq, Ok(messages));
        state
    }

    fn render_to_text(state: &AppState, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        // `draw()`'s return value carries the buffer that was just rendered
        // into — NOT `current_buffer_mut()` afterward, which by then points
        // at the *other*, freshly-reset buffer post-swap.
        let frame = terminal.draw(|frame| render(frame, state)).unwrap();
        let buffer = frame.buffer;
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn overflowing_history_shows_the_latest_messages_by_default() {
        let state = logged_in_state_with_messages(20);
        let content = render_to_text(&state, 60, 10);
        assert!(
            content.contains("msg-19"),
            "should show the latest message:\n{content}"
        );
        assert!(
            !content.contains("msg-0"),
            "should not show the oldest message without scrolling:\n{content}"
        );
    }

    #[test]
    fn page_up_reveals_older_messages() {
        let mut state = logged_in_state_with_messages(20);
        // Enough presses to reach the very top regardless of the exact step size.
        for _ in 0..5 {
            state.on_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
        }
        let content = render_to_text(&state, 60, 10);
        assert!(
            content.contains("msg-0"),
            "scrolling up should reveal the oldest message:\n{content}"
        );
    }

    /// Not a pass/fail check — dumps a rendered frame with a representative
    /// mix of formatting (bold/italic/code/links/mentions/lists/blockquote/
    /// attachments) straight to this terminal's real color output, so a
    /// styling change can be eyeballed without needing a real account.
    /// `#[ignore]`d like the keyring test; run explicitly:
    /// `cargo test visual_preview -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn visual_preview_of_rich_rendering() {
        use crate::api::types::Channel;

        let mut state = AppState::resuming();
        state.on_resume_finished(Some(User {
            id: "u1".into(),
            name: "Chris".into(),
            email: "chris@example.com".into(),
            approval_status: "approved".into(),
        }));
        let channels = vec![Channel {
            id: "c1".into(),
            name: "General".into(),
            description: None,
            is_private: false,
            is_archived: false,
            is_favorite: false,
            unread_count: 0,
        }];
        let Some(Command::LoadMessages { seq, .. }) = state.on_channels_loaded(Ok(channels)) else {
            panic!("expected the initial channel load to request messages");
        };
        let author = |id: &str, name: &str| MessageAuthor {
            id: id.to_string(),
            name: name.to_string(),
            preferences: None,
        };
        let msg = |id: &str, author: MessageAuthor, body: &str| Message {
            id: id.to_string(),
            kind: "user".to_string(),
            body: body.to_string(),
            created_at: Utc::now(),
            deleted_at: None,
            author,
            attachments: Vec::new(),
        };
        let messages = vec![
            msg(
                "m1",
                author("u1", "Chris"),
                "<p>Hey <strong>everyone</strong>, check <em>this</em> out</p>",
            ),
            msg(
                "m2",
                author("u2", "Louise"),
                r#"<p>Found it: <a href="https://example.com/recipe">the recipe</a> — <code>preheat to 220C</code></p>"#,
            ),
            msg(
                "m3",
                author("u1", "Chris"),
                r#"<p>cc <span data-type="mention" data-id="u2" data-label="Louise">@Louise</span> — shopping list:</p><ul><li>eggs</li><li>flour</li></ul>"#,
            ),
            msg(
                "m4",
                author("u3", "Grandma"),
                "<blockquote><p>back in my day we wrote recipes on paper</p></blockquote>",
            ),
            Message {
                attachments: vec![crate::api::types::Attachment {
                    kind: "image".to_string(),
                    secure_url: "https://res.cloudinary.com/example/sunset.jpg".to_string(),
                    width: Some(1920),
                    height: Some(1080),
                    original_filename: Some("sunset.jpg".to_string()),
                }],
                ..msg("m5", author("u2", "Louise"), "<p>view from the porch</p>")
            },
        ];
        state.on_messages_loaded("c1".to_string(), seq, Ok(messages));

        // Raw mode + alternate screen (reusing the app's own lifecycle, not
        // a bare CrosstermBackend) — otherwise this draws over whatever
        // cargo's own build/test output already put on the terminal, which
        // corrupts the preview with leftover text from *that*, not this.
        let mut terminal = super::super::init_terminal().unwrap();
        terminal.draw(|frame| render(frame, &state)).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(8));
        super::super::restore_terminal().unwrap();
    }
}
