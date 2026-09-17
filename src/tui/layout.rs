//! Top-level pane layout (#24/#50).

use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub sidebar: Rect,
    pub main: Rect,
    pub status: Rect,
}

pub fn split(area: Rect) -> AppLayout {
    let [body, status] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .areas(area);
    let [sidebar, main] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(0)])
        .areas(body);
    AppLayout {
        sidebar,
        main,
        status,
    }
}
