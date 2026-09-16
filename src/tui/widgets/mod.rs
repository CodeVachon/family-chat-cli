//! Channel list, message pane, composer, and auth-status widgets (#24/#26).

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::AppState;
use crate::app::state::{LoggedInState, LoginField, LoginForm, Screen};
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
    let style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(style);
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
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title("Channels")),
        layout.sidebar,
    );

    let title = state
        .selected_channel()
        .map(|c| format!("# {}", c.name))
        .unwrap_or_else(|| "family-chat".to_string());
    let body = if channels.is_empty() {
        "No channels yet.".to_string()
    } else if state.loading_messages {
        "Loading messages…".to_string()
    } else {
        match state
            .selected_channel()
            .and_then(|c| state.messages.get(&c.id))
        {
            Some(messages) if messages.is_empty() => "No messages yet.".to_string(),
            Some(messages) => messages
                .iter()
                .map(|message| {
                    format!(
                        "[{}] {}: {}",
                        message.created_at.format("%H:%M"),
                        message.author.display_name(),
                        html::to_plain_text(&message.body)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            None => String::new(),
        }
    };
    frame.render_widget(
        Paragraph::new(body)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: false }),
        layout.main,
    );

    let status = state.status.clone().unwrap_or_else(|| {
        format!(
            "Signed in as {} · \u{2191}/\u{2193} channels · r refresh · l logout · q quit",
            state.user.name
        )
    });
    frame.render_widget(Paragraph::new(status), layout.status);
}
