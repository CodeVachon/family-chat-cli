//! Channel list, message pane, composer, and auth-status widgets (#24/#26).

use chrono::{Local, NaiveDate};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::api::types::{Attachment, Channel, Message, MessageAuthor};
use crate::app::AppState;
use crate::app::state::{LoggedInFocus, LoggedInState, LoginField, LoginForm, Screen};
use crate::text::html;

use super::layout::split;

/// Below this, the two fixed-width side columns (28-col channels sidebar +
/// 24-col users pane, see `layout::split`) alone leave little to nothing
/// for the main pane, and message text wraps into an unreadable
/// single-word-per-line column — confirmed by rendering the real layout at
/// a range of sizes (see the `dump_small_sizes` test) rather than picking a
/// number blind. Re-derived (50 → 80) when the users pane (#50) added a
/// second fixed-width column, since the old floor left the main pane at
/// zero width. Nothing panics below this floor either way (ratatui's
/// constraint solver degrades gracefully, see
/// `tiny_terminal_sizes_never_panic`) — this is purely so a too-small
/// terminal gets one clear message instead of an unusably squeezed layout.
const MIN_WIDTH: u16 = 80;
const MIN_HEIGHT: u16 = 12;

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        render_too_small(frame, area);
        return;
    }
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

/// No border, no centering rect — at the very smallest sizes (down to 1x1,
/// see `tiny_terminal_sizes_never_panic`) even a bordered box wouldn't
/// necessarily fit. `Wrap` lets this degrade to whatever's actually visible
/// rather than clipping mid-word.
fn render_too_small(frame: &mut Frame, area: Rect) {
    let message = format!(
        "Terminal too small ({}x{}) — resize to at least {MIN_WIDTH}x{MIN_HEIGHT}.",
        area.width, area.height
    );
    frame.render_widget(
        Paragraph::new(message)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        area,
    );
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
    let sidebar_style = focus_style(state.focus == LoggedInFocus::Channels);
    // A load failure is pane-local (#26): shown inside the channels pane
    // itself, not a shared status line with no indication of which request
    // failed. Only *replaces* the list when there's nothing cached yet —
    // a background reload (resync, manual refresh) failing must never blank
    // an already-loaded channel list, just like the anti-flash fix for the
    // message pane. When channels are still showing, the title gets a short,
    // fixed marker instead of the full (unbounded-length) error text — the
    // sidebar's width is a fixed 28 columns (see layout::split), too narrow
    // for most error messages, and a title can't wrap the way pane content
    // can. The full message is still in the log file (see tui::log_if_err).
    let sidebar_title = match &state.channels_error {
        Some(_) if !channels.is_empty() => "Channels — error".to_string(),
        _ => "Channels".to_string(),
    };
    if let Some(error) = &state.channels_error {
        if channels.is_empty() {
            frame.render_widget(
                Paragraph::new(error.as_str())
                    .style(Style::default().fg(Color::Red))
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(sidebar_title)
                            .style(sidebar_style),
                    ),
                layout.sidebar,
            );
        } else {
            render_channel_list(frame, layout.sidebar, state, channels, sidebar_title);
        }
    } else {
        render_channel_list(frame, layout.sidebar, state, channels, sidebar_title);
    }

    let selected_channel_title = state
        .selected_channel()
        .map(|c| format!("# {}", c.name))
        .unwrap_or_else(|| "family-chat".to_string());
    let cached_messages = state
        .selected_channel()
        .and_then(|c| state.messages.get(&c.id));
    // Same pane-local principle as the channels error above: only replace
    // the message pane's content with the error when there's nothing cached
    // to fall back on; otherwise note it with a short, fixed title marker
    // (same width reasoning as the channels pane) and keep showing what was
    // already loaded.
    let messages_title = match &state.messages_error {
        Some(_) if cached_messages.is_some() => {
            format!("{selected_channel_title} — error")
        }
        _ => selected_channel_title,
    };
    let body: Vec<Line> = if channels.is_empty() {
        vec![Line::from("No channels yet.")]
    } else if let Some(error) = &state.messages_error {
        if cached_messages.is_none() {
            vec![Line::styled(
                error.as_str().to_string(),
                Style::default().fg(Color::Red),
            )]
        } else {
            windowed_messages(cached_messages, messages_area, state.message_scroll)
        }
    } else if state.loading_messages && cached_messages.is_none() {
        // Only show the loading placeholder when there's nothing cached yet
        // for this channel — a reload after sending, switching back to an
        // already-seen channel, or a realtime resync should update the pane
        // quietly once the fresh page arrives, not blank out messages that
        // are still perfectly valid to keep showing meanwhile.
        vec![Line::from("Loading messages…")]
    } else {
        match cached_messages {
            Some(messages) if messages.is_empty() => vec![Line::from("No messages yet.")],
            Some(_) => windowed_messages(cached_messages, messages_area, state.message_scroll),
            None => vec![],
        }
    };
    frame.render_widget(
        Paragraph::new(Text::from(body))
            .block(Block::default().borders(Borders::ALL).title(messages_title))
            .wrap(Wrap { trim: false }),
        messages_area,
    );

    let compose_focused = state.focus == LoggedInFocus::Compose;
    // Same short-marker-in-the-title reasoning as the channels/messages
    // panes above — the compose box is only 1 line tall inside its border,
    // with no room to wrap a full error message. The draft itself is never
    // lost (see #31), and the full message is in the log file.
    let compose_title = match (&state.send_error, state.sending) {
        (Some(_), _) => "Message — send failed".to_string(),
        (None, true) => "Sending…".to_string(),
        (None, false) => "Message".to_string(),
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

    render_users(frame, layout.users, state);

    let hint = format!(
        "Signed in as {} · Tab switch focus · \u{2191}/\u{2193} channels · PgUp/PgDn scroll · Enter send · r refresh · l logout · q quit",
        state.user.name
    );
    frame.render_widget(Paragraph::new(hint), layout.status);
}

/// The current channel's member list (#50) — read-only, no selection/focus
/// of its own (nothing in the app yet acts on a specific selected member;
/// see the ticket's note for why this stayed a display-only pane). An
/// online marker is shown only for ids the last `presence.snapshot`
/// actually reported; presence for anyone else is unknown, not "offline" —
/// see `RealtimeEvent::PresenceSnapshot`'s doc comment for why there's no
/// finer-grained update than that snapshot.
fn render_users(frame: &mut Frame, area: Rect, state: &LoggedInState) {
    let no_channels = state.channels.as_ref().is_none_or(Vec::is_empty);
    let members = state
        .selected_channel()
        .and_then(|c| state.members.get(&c.id));
    let body: Vec<Line> = if no_channels {
        vec![Line::from("No channels yet.")]
    } else {
        match (&state.members_error, members) {
            (Some(error), None) => vec![Line::styled(
                error.as_str().to_string(),
                Style::default().fg(Color::Red),
            )],
            (_, Some(members)) if members.is_empty() => vec![Line::from("No users.")],
            (_, Some(members)) => members
                .iter()
                .map(|member| {
                    let online = state.online_user_ids.contains(&member.user_id);
                    let marker = Span::styled(
                        if online { "● " } else { "○ " },
                        Style::default().fg(if online {
                            Color::Green
                        } else {
                            Color::DarkGray
                        }),
                    );
                    Line::from(vec![marker, Span::raw(member.name.clone())])
                })
                .collect(),
            (None, None) => vec![Line::from("Loading users…")],
        }
    };
    frame.render_widget(
        Paragraph::new(Text::from(body))
            .block(Block::default().borders(Borders::ALL).title("Users"))
            .wrap(Wrap { trim: false }),
        area,
    );
}

/// A bordered pane's inner content height.
fn visible_rows(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize
}

/// A bordered pane's inner content width — needed to know how many rendered
/// rows a wrapped `Line` will actually take (see `windowed`).
fn visible_cols(area: Rect) -> usize {
    area.width.saturating_sub(2) as usize
}

/// The suffix of `lines` that fits within `height` *rendered* rows at `width`
/// columns, offset upward by `scroll` lines — i.e. what a bottom-anchored,
/// scroll-up-for-history pane shows.
///
/// Counts each line's actual *wrapped* row count (`Line::width()` divided by
/// `width`, rounding up), not 1 row per logical line. Getting this wrong is
/// exactly what caused a real bug: a long attachment URL wraps to 2 rendered
/// rows, but counting it as 1 reserved one row too few — silently pushing
/// genuinely newer messages below the pane's bottom edge, where `Paragraph`
/// just clips whatever doesn't fit its rect with no error and no visible
/// sign anything was cut. That looked exactly like "the TUI has stale data"
/// even though the fetch itself was always current.
///
/// `scroll`'s own clamp still treats `height` as a line-count proxy (not
/// wrap-aware) — a much more benign approximation, since it only affects how
/// far you can page up, not whether the default (unscrolled) view clips
/// real content.
fn windowed<'a>(lines: Vec<Line<'a>>, width: usize, height: usize, scroll: usize) -> Vec<Line<'a>> {
    let scroll = scroll.min(lines.len().saturating_sub(height));
    let end = lines.len() - scroll;

    let width = width.max(1);
    let mut start = end;
    let mut rows_used = 0usize;
    while start > 0 {
        let rows = lines[start - 1].width().div_ceil(width).max(1);
        if rows_used + rows > height {
            break;
        }
        rows_used += rows;
        start -= 1;
    }

    lines[start..end].to_vec()
}

/// `messages`, rendered and windowed to fit `area` at the given scroll
/// offset — the shared tail end of both the normal path and the
/// error-with-cached-content path in `logged_in_view` (see `windowed`'s doc
/// comment for why wrapped row count, not line count, matters here).
fn windowed_messages(
    messages: Option<&Vec<Message>>,
    area: Rect,
    scroll: usize,
) -> Vec<Line<'static>> {
    let Some(messages) = messages else {
        return vec![];
    };
    windowed(
        messages_to_lines(messages),
        visible_cols(area),
        visible_rows(area),
        scroll,
    )
}

/// The channel list, rendered as-is — split out so the channels pane can
/// still show a previously-loaded list even when a later background reload
/// fails (see `logged_in_view`'s `channels_error` handling).
fn render_channel_list(
    frame: &mut Frame,
    area: Rect,
    state: &LoggedInState,
    channels: &[Channel],
    title: String,
) {
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
                .title(title)
                .style(sidebar_style),
        ),
        area,
    );
}

/// All of a channel's messages, with a `YYYY-MM-DD` divider inserted (in the
/// local timezone) wherever the calendar date changes — messages here can
/// span weeks, and a bare `HH:MM` gives no way to tell which day is which.
fn messages_to_lines(messages: &[Message]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut last_date: Option<NaiveDate> = None;

    for message in messages {
        let date = message.created_at.with_timezone(&Local).date_naive();
        if last_date != Some(date) {
            lines.push(date_divider(date));
            last_date = Some(date);
        }
        lines.extend(message_lines(message));
    }

    lines
}

fn date_divider(date: NaiveDate) -> Line<'static> {
    Line::from(Span::styled(
        format!("── {} ──", date.format("%Y-%m-%d")),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))
}

/// One message: an author/timestamp-prefixed rendering of its (rich-text)
/// body, followed by one line per attachment.
fn message_lines(message: &Message) -> Vec<Line<'static>> {
    let prefix = vec![
        Span::styled(
            format!(
                "[{}] ",
                message.created_at.with_timezone(&Local).format("%H:%M")
            ),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("{}: ", message.author.display_name()),
            Style::default()
                .fg(author_color(&message.author))
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let mut lines = if message.kind == "system" {
        vec![Line::from(Span::styled(
            system_event_text(message),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ))]
    } else {
        html::to_lines(&message.body)
    };
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

/// A channel-membership event's description, rendered after the usual
/// `[HH:MM] Author:` prefix (where `Author` is the *subject* of the event —
/// the server sets the message's `author` to whoever joined/left, not
/// whoever performed the action). We only have the actor's raw id, not
/// their display name, so actor-initiated events on someone else don't name
/// the actor.
fn system_event_text(message: &Message) -> String {
    let Some(event) = &message.system_event else {
        return "performed a channel action".to_string();
    };
    let is_self = event.subject_user_id.as_deref() == Some(event.actor_user_id.as_str());
    match event.event.as_str() {
        "join" if is_self => "joined the channel".to_string(),
        "join" => "was added to the channel".to_string(),
        "leave" if is_self => "left the channel".to_string(),
        "leave" => "was removed from the channel".to_string(),
        other => other.replace('_', " "),
    }
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
    use crate::api::types::SystemEvent;

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

    /// Wide enough that none of these short single-character lines wrap —
    /// isolates the tests below from `windowed`'s wrap-awareness, which gets
    /// its own dedicated tests further down.
    const WIDE: usize = 80;

    #[test]
    fn shows_the_tail_when_content_overflows_the_pane() {
        assert_eq!(
            labels(&windowed(lines(&["1", "2", "3", "4", "5"]), WIDE, 3, 0)),
            ["3", "4", "5"]
        );
    }

    #[test]
    fn scrolling_up_shows_older_lines() {
        assert_eq!(
            labels(&windowed(lines(&["1", "2", "3", "4", "5"]), WIDE, 3, 2)),
            ["1", "2", "3"]
        );
    }

    #[test]
    fn scrolling_past_the_start_clamps_rather_than_panics() {
        assert_eq!(
            labels(&windowed(lines(&["1", "2", "3"]), WIDE, 3, 100)),
            ["1", "2", "3"]
        );
    }

    #[test]
    fn short_content_is_shown_in_full() {
        assert_eq!(
            labels(&windowed(lines(&["1", "2"]), WIDE, 10, 0)),
            ["1", "2"]
        );
    }

    #[test]
    fn a_wide_line_reserves_its_actual_wrapped_row_count() {
        // Regression test for a real bug: a long attachment URL (or any
        // line wider than the pane) wraps to multiple rendered rows, but
        // the old windowed() counted it as a single row — reserving one
        // row too few and silently pushing the genuinely newest message
        // below the pane's bottom edge, where Paragraph just clips
        // whatever doesn't fit its rect. There's no error and no visible
        // sign anything was cut — it just looks like stale data.
        //
        // At width 10, "this-line-is-wider-than-ten" (27 chars) needs 3
        // wrapped rows. With height 3, the old (buggy) formula would have
        // requested all 3 lines (one line == one row, 3 lines <= height 3)
        // — a real Paragraph would then have no rows left for "newest" at
        // all once "one" (row 0) and the wide line's wrap (rows 1-2, of
        // the 3 it actually needs) ate the pane's only 3 rows.
        let lines = vec![
            Line::from("one"),
            Line::from("this-line-is-wider-than-ten"),
            Line::from("newest"),
        ];
        let result = windowed(lines, 10, 3, 0);
        assert_eq!(
            labels(&result),
            ["newest"],
            "the wide line alone needs all 3 rows once truly reserved, so \
             only the newest line — never dropped — fits alongside it"
        );
    }

    // Full-render checks against a synthetic (never real-account) fixture —
    // this is what actually caught that overflowing history was clipping
    // the newest messages off the bottom with no way to scroll to them.

    use chrono::Utc;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::api::types::{Channel, ChannelMember, Message, MessageAuthor, User};
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
                system_event: None,
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
        state.on_messages_loaded("c1".to_string(), seq, Ok((messages, false)));
        state
    }

    fn logged_in_state_mut(state: &mut AppState) -> &mut LoggedInState {
        match &mut state.screen {
            Screen::LoggedIn(logged_in) => logged_in,
            _ => panic!("expected LoggedIn"),
        }
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
        let content = render_to_text(&state, 110, 15);
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
        let content = render_to_text(&state, 110, 15);
        assert!(
            content.contains("msg-0"),
            "scrolling up should reveal the oldest message:\n{content}"
        );
    }

    // Pane-local errors (#26) — a load/send failure must show up inside the
    // pane it actually belongs to, and the shared bottom line must always
    // keep showing the key-hints text, never absorb an error itself.

    #[test]
    fn a_channels_error_with_nothing_cached_replaces_the_channel_list() {
        let mut state = logged_in_state_with_messages(1);
        logged_in_state_mut(&mut state).channels = None;
        logged_in_state_mut(&mut state).channels_error =
            Some("Couldn't load channels: offline".to_string());

        // Checked as two separate substrings, not one contiguous phrase —
        // the error wraps onto its own line inside the narrow sidebar, so
        // the rendered text has a newline where the source string had a
        // space.
        let content = render_to_text(&state, 110, 15);
        assert!(
            content.contains("Couldn't load channels"),
            "the channels error should render inside the channels pane:\n{content}"
        );
        assert!(content.contains("offline"));
    }

    #[test]
    fn a_channels_error_with_a_cached_list_keeps_showing_the_list() {
        let mut state = logged_in_state_with_messages(1);
        logged_in_state_mut(&mut state).channels_error =
            Some("Couldn't load channels: offline".to_string());

        let content = render_to_text(&state, 110, 15);
        assert!(
            content.contains("General"),
            "a background refresh failure must not blank an already-loaded channel list:\n{content}"
        );
        assert!(
            content.contains("Channels — error"),
            "the pane's title should note the failure (short marker — the \
             sidebar is too narrow to fit an arbitrary-length message):\n{content}"
        );
    }

    #[test]
    fn a_messages_error_with_nothing_cached_replaces_the_message_pane() {
        let mut state = logged_in_state_with_messages(0);
        logged_in_state_mut(&mut state).messages.remove("c1");
        logged_in_state_mut(&mut state).messages_error =
            Some("Couldn't load messages: offline".to_string());

        // Same wrap-safe check as the channels-pane test above.
        let content = render_to_text(&state, 110, 15);
        assert!(
            content.contains("Couldn't load messages"),
            "the messages error should render inside the messages pane:\n{content}"
        );
        assert!(content.contains("offline"));
    }

    #[test]
    fn a_messages_error_with_cached_messages_keeps_showing_them() {
        let mut state = logged_in_state_with_messages(3);
        logged_in_state_mut(&mut state).messages_error =
            Some("Couldn't load older messages: offline".to_string());

        let content = render_to_text(&state, 110, 15);
        assert!(
            content.contains("msg-2"),
            "a failed background/older-page load must not blank already-loaded messages:\n{content}"
        );
        assert!(
            content.contains("— error"),
            "the pane's title should note the failure (short marker — a \
             channel name plus an arbitrary-length error won't reliably fit):\n{content}"
        );
    }

    #[test]
    fn a_send_error_shows_in_the_compose_title_and_keeps_the_draft() {
        let mut state = logged_in_state_with_messages(0);
        {
            let logged_in = logged_in_state_mut(&mut state);
            logged_in.compose = "hello".to_string();
            logged_in.send_error = Some("Couldn't send message: offline".to_string());
        }

        let content = render_to_text(&state, 110, 15);
        assert!(
            content.contains("hello"),
            "a failed send must never lose the draft:\n{content}"
        );
        assert!(
            content.contains("send failed"),
            "the compose box's title should note the failure:\n{content}"
        );
    }

    #[test]
    fn the_bottom_line_always_shows_the_hints_never_an_error() {
        let mut state = logged_in_state_with_messages(1);
        {
            let logged_in = logged_in_state_mut(&mut state);
            logged_in.channels_error = Some("channels boom".to_string());
            logged_in.messages_error = Some("messages boom".to_string());
            logged_in.send_error = Some("send boom".to_string());
        }

        let content = render_to_text(&state, 110, 15);
        assert!(
            content.contains("Tab switch focus"),
            "the shared bottom line should still show the key hints:\n{content}"
        );
    }

    #[test]
    fn the_users_pane_shows_members_with_an_online_marker_from_presence() {
        let mut state = logged_in_state_with_messages(1);
        {
            let logged_in = logged_in_state_mut(&mut state);
            logged_in.members.insert(
                "c1".to_string(),
                vec![
                    ChannelMember {
                        user_id: "u1".to_string(),
                        role: "owner".to_string(),
                        name: "Louise".to_string(),
                        color_hue: Some(220),
                        avatar_url: None,
                    },
                    ChannelMember {
                        user_id: "u2".to_string(),
                        role: "user".to_string(),
                        name: "Rachel".to_string(),
                        color_hue: Some(163),
                        avatar_url: None,
                    },
                ],
            );
            logged_in.online_user_ids.insert("u1".to_string());
        }

        let content = render_to_text(&state, 110, 15);
        assert!(content.contains("Louise"));
        assert!(content.contains("Rachel"));
        assert!(
            content.contains("● Louise"),
            "the online member should get the filled marker:\n{content}"
        );
        assert!(
            content.contains("○ Rachel"),
            "a member with unknown/offline presence should get the hollow marker:\n{content}"
        );
    }

    #[test]
    fn the_users_pane_shows_loading_until_members_arrive() {
        let state = logged_in_state_with_messages(1);
        let content = render_to_text(&state, 110, 15);
        assert!(content.contains("Loading users"));
    }

    #[test]
    fn below_the_minimum_size_shows_a_resize_message_instead_of_the_ui() {
        let state = logged_in_state_with_messages(5);
        let content = render_to_text(&state, MIN_WIDTH - 1, MIN_HEIGHT);
        assert!(
            content.contains("Terminal too small"),
            "a too-narrow terminal should show the resize message:\n{content}"
        );
        assert!(!content.contains("Channels"));

        let content = render_to_text(&state, MIN_WIDTH, MIN_HEIGHT - 1);
        assert!(
            content.contains("Terminal too small"),
            "a too-short terminal should show the resize message:\n{content}"
        );
    }

    #[test]
    fn at_the_minimum_size_the_normal_ui_renders() {
        let state = logged_in_state_with_messages(5);
        let content = render_to_text(&state, MIN_WIDTH, MIN_HEIGHT);
        assert!(
            !content.contains("Terminal too small"),
            "exactly the minimum size should be treated as usable:\n{content}"
        );
        assert!(content.contains("Channels"));
    }

    #[test]
    fn tiny_terminal_sizes_never_panic() {
        // ratatui's own Layout constraint solver degrades gracefully at
        // small sizes rather than panicking, but this pins that down for
        // every screen this app actually renders, not just relying on
        // ratatui's own guarantees (#28).
        for (w, h) in [(1u16, 1u16), (2, 2), (5, 3), (10, 5), (20, 8)] {
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();

            let logged_out = AppState::resuming();
            terminal.draw(|frame| render(frame, &logged_out)).unwrap();

            let with_messages = logged_in_state_with_messages(5);
            terminal
                .draw(|frame| render(frame, &with_messages))
                .unwrap();
        }
    }

    /// Not a pass/fail check — dumps the rendered frame at a range of sizes
    /// so the actual on-screen result (not just "did it panic") can be
    /// eyeballed to pick a sensible minimum-size floor. Run explicitly:
    /// `cargo test dump_small_sizes -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn dump_small_sizes() {
        for (w, h) in [(100u16, 24u16), (80, 18), (80, 12), (70, 16), (50, 12)] {
            let state = logged_in_state_with_messages(5);
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            let frame = terminal.draw(|frame| render(frame, &state)).unwrap();
            let buffer = &frame.buffer;
            println!("=== {w}x{h} ===");
            for y in 0..buffer.area.height {
                let line: String = (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect();
                println!("{line}");
            }
        }
    }

    #[test]
    fn a_date_divider_appears_once_per_calendar_day_not_per_message() {
        use chrono::TimeZone;

        let make = |id: &str, created_at: chrono::DateTime<Utc>| Message {
            id: id.to_string(),
            kind: "user".to_string(),
            system_event: None,
            body: format!("<p>{id}</p>"),
            created_at,
            deleted_at: None,
            author: MessageAuthor {
                id: "u1".to_string(),
                name: "Chris".to_string(),
                preferences: None,
            },
            attachments: Vec::new(),
        };
        let day1 = Utc.with_ymd_and_hms(2026, 9, 14, 10, 0, 0).unwrap();
        let day2 = Utc.with_ymd_and_hms(2026, 9, 16, 10, 0, 0).unwrap();
        let messages = vec![make("a", day1), make("b", day1), make("c", day2)];

        let lines = messages_to_lines(&messages);
        let divider_count = lines
            .iter()
            .filter(|line| line.spans.iter().any(|s| s.content.contains("──")))
            .count();

        assert_eq!(
            divider_count, 2,
            "two distinct days should get exactly two dividers, not one per message"
        );
    }

    #[test]
    fn a_self_join_system_event_describes_itself_after_the_author_prefix() {
        let message = Message {
            id: "sys1".to_string(),
            kind: "system".to_string(),
            system_event: Some(SystemEvent {
                event: "join".to_string(),
                actor_user_id: "u1".to_string(),
                subject_user_id: Some("u1".to_string()),
            }),
            body: String::new(),
            created_at: Utc::now(),
            deleted_at: None,
            author: MessageAuthor {
                id: "u1".to_string(),
                name: "Christopher".to_string(),
                preferences: None,
            },
            attachments: Vec::new(),
        };

        let rendered: String = message_lines(&message)
            .into_iter()
            .flat_map(|line| line.spans.into_iter().map(|s| s.content.into_owned()))
            .collect();

        assert!(
            rendered.ends_with("Christopher: joined the channel"),
            "a self-join should render as \"joined the channel\" after the usual prefix, got: {rendered}"
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
            system_event: None,
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
        state.on_messages_loaded("c1".to_string(), seq, Ok((messages, false)));

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
