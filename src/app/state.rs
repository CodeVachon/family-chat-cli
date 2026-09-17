//! State + reducer-style transitions (#12). No IO, so directly unit-testable (#42).

use std::collections::{HashMap, HashSet};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::api::ApiError;
use crate::api::types::{Channel, Message, RealtimeEvent, User};

use super::event::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginField {
    Email,
    Password,
}

#[derive(Debug, Clone, Default)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    pub focus: Option<LoginField>,
    pub error: Option<String>,
}

impl LoginForm {
    fn new() -> Self {
        Self {
            focus: Some(LoginField::Email),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedInFocus {
    Channels,
    Compose,
}

#[derive(Debug)]
pub struct LoggedInState {
    pub user: User,
    /// `None` until the initial `GET /channels` call returns.
    pub channels: Option<Vec<Channel>>,
    pub selected: usize,
    pub messages: HashMap<String, Vec<Message>>,
    pub loading_messages: bool,
    /// Pane-local errors (#26) — shown inside whichever pane the failure
    /// actually belongs to, rather than one shared status line that gave no
    /// indication of which request had failed.
    pub channels_error: Option<String>,
    pub messages_error: Option<String>,
    pub send_error: Option<String>,
    pub focus: LoggedInFocus,
    pub compose: String,
    pub sending: bool,
    /// Lines scrolled up from the latest message (0 = showing the latest —
    /// see `tui::widgets` for how this windows the rendered history). Reset
    /// to 0 whenever the selected channel changes or fresh messages land, so
    /// the pane always opens on the newest content, matching ordinary chat
    /// UX (see #49).
    pub message_scroll: usize,
    /// The most recent request sequence number issued per channel — lets
    /// `on_messages_loaded` reject a stale response that lost the race
    /// against a newer request for the same channel (manual refresh, a
    /// channel switch, and an SSE-triggered reload can all fire close
    /// together — see #51).
    message_request_seq: HashMap<String, u64>,
    next_seq: u64,
    /// Per channel: whether the server has even older messages beyond what's
    /// cached. Absent (not yet known) is treated as "maybe" — safe to try
    /// once. Set from every `(latest page)` and `(older page)` response,
    /// since both carry the server's `hasMore` for that call (#30).
    has_more: HashMap<String, bool>,
    /// Channels with an older-page fetch currently in flight — guards
    /// against firing a second one before the first resolves.
    loading_older: HashSet<String>,
}

impl LoggedInState {
    fn new(user: User) -> Self {
        Self {
            user,
            channels: None,
            selected: 0,
            messages: HashMap::new(),
            loading_messages: false,
            channels_error: None,
            messages_error: None,
            send_error: None,
            focus: LoggedInFocus::Channels,
            compose: String::new(),
            sending: false,
            message_scroll: 0,
            message_request_seq: HashMap::new(),
            next_seq: 0,
            has_more: HashMap::new(),
            loading_older: HashSet::new(),
        }
    }

    pub fn selected_channel(&self) -> Option<&Channel> {
        self.channels.as_ref().and_then(|c| c.get(self.selected))
    }

    /// Builds a `LoadMessages` command and records it as the latest
    /// outstanding request for `channel_id`, so a response can later be
    /// checked for staleness against whatever request superseded it.
    fn request_messages(&mut self, channel_id: String) -> Command {
        self.next_seq += 1;
        self.message_request_seq
            .insert(channel_id.clone(), self.next_seq);
        Command::LoadMessages {
            channel_id,
            seq: self.next_seq,
        }
    }
}

#[derive(Debug)]
pub enum Screen {
    /// Trying to resume a session from a stored credential before showing
    /// anything else.
    Resuming,
    LoggedOut(LoginForm),
    LoggingIn,
    /// Authenticated, but the account isn't `approved` yet (see
    /// docs/api-contract.md — pending or rejected, the server doesn't
    /// distinguish in the message).
    NotApproved,
    LoggedIn(Box<LoggedInState>),
}

#[derive(Debug)]
pub struct AppState {
    pub screen: Screen,
    pub should_quit: bool,
}

impl AppState {
    pub fn resuming() -> Self {
        Self {
            screen: Screen::Resuming,
            should_quit: false,
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) -> Option<Command> {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return Some(Command::Quit);
        }

        match &mut self.screen {
            Screen::Resuming => None,
            Screen::LoggedOut(form) => Self::on_login_key(form, key),
            Screen::LoggingIn => None,
            Screen::NotApproved => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.should_quit = true;
                    Some(Command::Quit)
                }
                _ => None,
            },
            Screen::LoggedIn(state) => Self::on_logged_in_key(state, key, &mut self.should_quit),
        }
    }

    fn on_login_key(form: &mut LoginForm, key: KeyEvent) -> Option<Command> {
        let field = form.focus.unwrap_or(LoginField::Email);
        let current = match field {
            LoginField::Email => &mut form.email,
            LoginField::Password => &mut form.password,
        };

        match key.code {
            KeyCode::Char(c) => {
                current.push(c);
                None
            }
            KeyCode::Backspace => {
                current.pop();
                None
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Up => {
                form.focus = Some(match field {
                    LoginField::Email => LoginField::Password,
                    LoginField::Password => LoginField::Email,
                });
                None
            }
            KeyCode::Enter => {
                if form.email.trim().is_empty() || form.password.is_empty() {
                    form.error = Some("Email and password are both required.".to_string());
                    return None;
                }
                Some(Command::SubmitLogin {
                    email: form.email.trim().to_string(),
                    password: std::mem::take(&mut form.password),
                })
            }
            _ => None,
        }
    }

    /// Lines moved per `PageUp`/`PageDown` — a fixed step rather than a true
    /// screen-height, since the pure state layer doesn't know the terminal
    /// size (that's `tui::widgets`' job at render time).
    const SCROLL_STEP: usize = 10;

    fn on_logged_in_key(
        state: &mut LoggedInState,
        key: KeyEvent,
        should_quit: &mut bool,
    ) -> Option<Command> {
        // Scrolling message history works from either focus — it's not text
        // input, so there's no ambiguity with typing into the composer.
        match key.code {
            KeyCode::PageUp => {
                state.message_scroll = state.message_scroll.saturating_add(Self::SCROLL_STEP);
                return Self::maybe_load_older(state);
            }
            KeyCode::PageDown => {
                state.message_scroll = state.message_scroll.saturating_sub(Self::SCROLL_STEP);
                return None;
            }
            _ => {}
        }

        if key.code == KeyCode::Tab {
            state.focus = match state.focus {
                LoggedInFocus::Channels => LoggedInFocus::Compose,
                LoggedInFocus::Compose => LoggedInFocus::Channels,
            };
            return None;
        }

        match state.focus {
            LoggedInFocus::Channels => Self::on_channels_key(state, key, should_quit),
            LoggedInFocus::Compose => Self::on_compose_key(state, key),
        }
    }

    fn on_channels_key(
        state: &mut LoggedInState,
        key: KeyEvent,
        should_quit: &mut bool,
    ) -> Option<Command> {
        match key.code {
            KeyCode::Char('q') => {
                *should_quit = true;
                Some(Command::Quit)
            }
            KeyCode::Char('l') => Some(Command::Logout),
            KeyCode::Char('r') => {
                let channel_id = state.selected_channel()?.id.clone();
                Some(state.request_messages(channel_id))
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected > 0 {
                    state.selected -= 1;
                    state.message_scroll = 0;
                    return Self::load_selected(state);
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = state.channels.as_ref().map_or(0, Vec::len);
                if state.selected + 1 < len {
                    state.selected += 1;
                    state.message_scroll = 0;
                    return Self::load_selected(state);
                }
                None
            }
            _ => None,
        }
    }

    /// The draft is kept in `state.compose` until a send is *confirmed*
    /// successful (see `on_message_sent`) — a failed or in-flight send must
    /// never lose what the user typed (#31).
    fn on_compose_key(state: &mut LoggedInState, key: KeyEvent) -> Option<Command> {
        match key.code {
            KeyCode::Char(c) => {
                state.compose.push(c);
                state.send_error = None;
                None
            }
            KeyCode::Backspace => {
                state.compose.pop();
                state.send_error = None;
                None
            }
            KeyCode::Enter => {
                if state.sending || state.compose.trim().is_empty() {
                    return None;
                }
                let channel_id = state.selected_channel()?.id.clone();
                state.sending = true;
                Some(Command::SendMessage {
                    channel_id,
                    body: state.compose.clone(),
                })
            }
            _ => None,
        }
    }

    /// Always re-fetches the latest page on switch, even if this channel was
    /// already cached — an earlier version skipped refreshing an
    /// already-cached channel, so messages posted while it wasn't selected
    /// (SSE only reloads the *currently viewed* channel — see
    /// `on_realtime_event`) would stay invisible until some other trigger
    /// (manual `r`, or that channel receiving *another* live event) came
    /// along. Switching to a channel is exactly the moment to make sure it's
    /// current.
    fn load_selected(state: &mut LoggedInState) -> Option<Command> {
        let channel_id = state.selected_channel()?.id.clone();
        Some(state.request_messages(channel_id))
    }

    /// Whether we've scrolled far enough up to plausibly be nearing the top
    /// of what's cached, and if so, requests the next older page — unless
    /// the server already said there's nothing older, or a fetch for this
    /// channel is already in flight.
    ///
    /// The trigger is `message_scroll >= cached message count`, using
    /// *messages* as a proxy for *rendered lines* (`message_scroll` is
    /// line-based; the state layer doesn't do HTML rendering, so it can't
    /// know the exact wrapped line count — that's `tui::widgets`' job).
    /// Since most chat messages render to one line, this fires at or
    /// slightly before the true boundary — an early prefetch, not a bug.
    fn maybe_load_older(state: &mut LoggedInState) -> Option<Command> {
        let channel_id = state.selected_channel()?.id.clone();
        let cached = state.messages.get(&channel_id)?;
        if state.message_scroll < cached.len() {
            return None;
        }
        if state.has_more.get(&channel_id) == Some(&false) {
            return None;
        }
        if !state.loading_older.insert(channel_id.clone()) {
            return None; // already fetching
        }
        let oldest = cached.first()?;
        Some(Command::LoadOlderMessages {
            channel_id,
            before_id: oldest.id.clone(),
            before_created_at: oldest.created_at,
        })
    }

    pub fn on_resume_finished(&mut self, user: Option<User>) -> Option<Command> {
        match user {
            Some(user) => {
                self.screen = Screen::LoggedIn(Box::new(LoggedInState::new(user)));
                Some(Command::LoadChannels)
            }
            None => {
                self.screen = Screen::LoggedOut(LoginForm::new());
                None
            }
        }
    }

    pub fn on_login_submitted(&mut self) {
        self.screen = Screen::LoggingIn;
    }

    pub fn on_login_finished(&mut self, result: Result<User, ApiError>) -> Option<Command> {
        match result {
            Ok(user) => {
                self.screen = Screen::LoggedIn(Box::new(LoggedInState::new(user)));
                Some(Command::LoadChannels)
            }
            Err(ApiError::NotApproved) => {
                self.screen = Screen::NotApproved;
                None
            }
            Err(error) => {
                let mut form = LoginForm::new();
                form.error = Some(error.to_string());
                self.screen = Screen::LoggedOut(form);
                None
            }
        }
    }

    pub fn on_channels_loaded(
        &mut self,
        result: Result<Vec<Channel>, ApiError>,
    ) -> Option<Command> {
        let Screen::LoggedIn(state) = &mut self.screen else {
            return None;
        };
        match result {
            Ok(channels) => {
                // Keep whatever was selected (clamped to the new list, which
                // may have shrunk/reordered) rather than always jumping back
                // to the first channel — this reload also fires on a live
                // `channels.changed`/`resync` event while the user is reading
                // something else.
                state.channels_error = None;
                state.selected = state.selected.min(channels.len().saturating_sub(1));
                state.channels = Some(channels);
                let channel_id = state.selected_channel().map(|channel| channel.id.clone());
                channel_id.map(|id| state.request_messages(id))
            }
            Err(error) => {
                state.channels_error = Some(format!("Couldn't load channels: {error}"));
                None
            }
        }
    }

    pub fn on_messages_loading(&mut self) {
        if let Screen::LoggedIn(state) = &mut self.screen {
            state.loading_messages = true;
        }
    }

    pub fn on_messages_loaded(
        &mut self,
        channel_id: String,
        seq: u64,
        result: Result<(Vec<Message>, bool), ApiError>,
    ) {
        let Screen::LoggedIn(state) = &mut self.screen else {
            return;
        };
        // A newer request for this same channel is still outstanding (or has
        // already been answered) — this response lost the race, so drop it
        // rather than let it clobber fresher data (#51).
        if state.message_request_seq.get(&channel_id) != Some(&seq) {
            return;
        }
        state.loading_messages = false;
        match result {
            Ok((messages, has_more)) => {
                state.messages_error = None;
                state.has_more.insert(channel_id.clone(), has_more);
                state.messages.insert(channel_id, messages);
                state.message_scroll = 0;
            }
            Err(error) => {
                state.messages_error = Some(format!("Couldn't load messages: {error}"));
            }
        }
    }

    /// The response to a `Command::LoadOlderMessages` (#30): prepends the
    /// fetched page to what's cached for `channel_id` (safe even if the user
    /// has since switched to a different channel — it just enriches that
    /// channel's cache for whenever they return) and records whether the
    /// server says there's still more beyond *that*.
    pub fn on_older_messages_loaded(
        &mut self,
        channel_id: String,
        result: Result<(Vec<Message>, bool), ApiError>,
    ) {
        let Screen::LoggedIn(state) = &mut self.screen else {
            return;
        };
        state.loading_older.remove(&channel_id);
        match result {
            Ok((mut older, has_more)) => {
                state.messages_error = None;
                state.has_more.insert(channel_id.clone(), has_more);
                if older.is_empty() {
                    return;
                }
                let mut added = older.len();
                if let Some(existing) = state.messages.get_mut(&channel_id) {
                    // The server's cursor is exclusive on a millisecond
                    // boundary and can hand back a row already in `existing`
                    // that shares the cursor's millisecond (see
                    // docs/api-contract.md) — drop those before merging
                    // rather than showing a duplicate line.
                    let existing_ids: HashSet<&str> =
                        existing.iter().map(|m| m.id.as_str()).collect();
                    older.retain(|m| !existing_ids.contains(m.id.as_str()));
                    added = older.len();
                    older.append(existing);
                    *existing = older;
                }
                // Keep the viewport anchored on what the user was already
                // reading rather than jumping now that older content exists
                // above it. Approximate (assumes ~1 rendered line per
                // message, usually right for short chat text) for the same
                // reason `maybe_load_older`'s trigger is approximate.
                if state.selected_channel().is_some_and(|c| c.id == channel_id) {
                    state.message_scroll = state.message_scroll.saturating_add(added);
                }
            }
            Err(error) => {
                state.messages_error = Some(format!("Couldn't load older messages: {error}"));
            }
        }
    }

    pub fn on_message_sent(
        &mut self,
        channel_id: String,
        result: Result<(), ApiError>,
    ) -> Option<Command> {
        let Screen::LoggedIn(state) = &mut self.screen else {
            return None;
        };
        state.sending = false;
        match result {
            Ok(()) => {
                state.compose.clear();
                state.send_error = None;
                // Force a reload rather than splicing the new message in
                // locally: the send response doesn't carry the decorated
                // shape (author/reactions/mentions) GET returns, and a
                // refetch is simple and correct without SSE to reconcile
                // against yet (see #57). Deliberately *not* clearing the
                // cached messages first — `request_messages` always
                // refetches regardless, and clearing here just blanked the
                // whole pane for the round trip, flashing the entire
                // channel on every send instead of updating quietly once
                // the fresh page arrives.
                Some(state.request_messages(channel_id))
            }
            Err(error) => {
                state.send_error = Some(format!("Couldn't send message: {error}"));
                None
            }
        }
    }

    pub fn on_logout(&mut self) {
        self.screen = Screen::LoggedOut(LoginForm::new());
    }

    /// A live event from `GET /api/v1/stream` (#57). Only reacts to the
    /// kinds this prototype actually renders (channels, messages) — anything
    /// else (typing, presence, reactions, mentions, read receipts,
    /// users/settings changes) is `RealtimeEvent::Other` and ignored here.
    pub fn on_realtime_event(&mut self, event: RealtimeEvent) -> Option<Command> {
        let Screen::LoggedIn(state) = &mut self.screen else {
            return None;
        };
        match event {
            RealtimeEvent::Ready | RealtimeEvent::Other => None,
            // A full reload naturally re-requests the selected channel's
            // messages too, via on_channels_loaded.
            RealtimeEvent::Resync | RealtimeEvent::ChannelsChanged => Some(Command::LoadChannels),
            RealtimeEvent::MessageCreated { channel_id }
            | RealtimeEvent::MessageUpdated { channel_id }
            | RealtimeEvent::MessageDeleted { channel_id } => {
                if state.selected_channel().is_some_and(|c| c.id == channel_id) {
                    Some(state.request_messages(channel_id))
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::api::types::MessageAuthor;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn typing_fills_the_focused_field() {
        let mut state = AppState::resuming();
        state.on_resume_finished(None);
        state.on_key(key(KeyCode::Char('a')));
        state.on_key(key(KeyCode::Char('b')));
        let Screen::LoggedOut(form) = &state.screen else {
            panic!("expected LoggedOut");
        };
        assert_eq!(form.email, "ab");
        assert_eq!(form.password, "");
    }

    #[test]
    fn tab_switches_focus_to_password() {
        let mut state = AppState::resuming();
        state.on_resume_finished(None);
        state.on_key(key(KeyCode::Char('a')));
        state.on_key(key(KeyCode::Tab));
        state.on_key(key(KeyCode::Char('x')));
        let Screen::LoggedOut(form) = &state.screen else {
            panic!("expected LoggedOut");
        };
        assert_eq!(form.email, "a");
        assert_eq!(form.password, "x");
    }

    #[test]
    fn enter_with_empty_password_sets_an_error_instead_of_submitting() {
        let mut state = AppState::resuming();
        state.on_resume_finished(None);
        state.on_key(key(KeyCode::Char('a')));
        let command = state.on_key(key(KeyCode::Enter));
        assert!(command.is_none());
        let Screen::LoggedOut(form) = &state.screen else {
            panic!("expected LoggedOut");
        };
        assert!(form.error.is_some());
    }

    #[test]
    fn enter_with_both_fields_submits_and_clears_password() {
        let mut state = AppState::resuming();
        state.on_resume_finished(None);
        state.on_key(key(KeyCode::Char('a')));
        state.on_key(key(KeyCode::Tab));
        state.on_key(key(KeyCode::Char('p')));
        let command = state.on_key(key(KeyCode::Enter));
        match command {
            Some(Command::SubmitLogin { email, password }) => {
                assert_eq!(email, "a");
                assert_eq!(password, "p");
            }
            other => panic!("expected SubmitLogin, got {other:?}"),
        }
    }

    #[test]
    fn not_approved_error_routes_to_its_own_screen() {
        let mut state = AppState::resuming();
        state.on_resume_finished(None);
        state.on_login_submitted();
        state.on_login_finished(Err(ApiError::NotApproved));
        assert!(matches!(state.screen, Screen::NotApproved));
    }

    #[test]
    fn channel_navigation_requests_messages_for_a_not_yet_loaded_channel() {
        let mut state = logged_in_with_channels(["General", "Random"]);

        let command = state.on_key(key(KeyCode::Down));
        match command {
            Some(Command::LoadMessages { channel_id, .. }) => assert_eq!(channel_id, "c2"),
            other => panic!("expected LoadMessages, got {other:?}"),
        }
    }

    #[test]
    fn tab_moves_focus_into_compose_where_letters_are_typed_not_shortcuts() {
        let mut state = logged_in_with_channels(["General"]);
        state.on_key(key(KeyCode::Tab));
        state.on_key(key(KeyCode::Char('q'))); // would quit in channel-nav focus

        assert!(!state.should_quit);
        let Screen::LoggedIn(logged_in) = &state.screen else {
            panic!("expected LoggedIn");
        };
        assert_eq!(logged_in.compose, "q");
    }

    #[test]
    fn enter_on_an_empty_draft_does_not_send() {
        let mut state = logged_in_with_channels(["General"]);
        state.on_key(key(KeyCode::Tab));
        let command = state.on_key(key(KeyCode::Enter));
        assert!(command.is_none());
    }

    #[test]
    fn enter_sends_and_a_second_enter_is_ignored_while_sending() {
        let mut state = logged_in_with_channels(["General"]);
        state.on_key(key(KeyCode::Tab));
        state.on_key(key(KeyCode::Char('h')));
        state.on_key(key(KeyCode::Char('i')));

        let first = state.on_key(key(KeyCode::Enter));
        match first {
            Some(Command::SendMessage { channel_id, body }) => {
                assert_eq!(channel_id, "c1");
                assert_eq!(body, "hi");
            }
            other => panic!("expected SendMessage, got {other:?}"),
        }

        // The draft is kept (not cleared) until the send is confirmed, and a
        // second Enter while `sending` is true must not fire another send.
        let Screen::LoggedIn(logged_in) = &state.screen else {
            panic!("expected LoggedIn");
        };
        assert_eq!(logged_in.compose, "hi");
        assert!(state.on_key(key(KeyCode::Enter)).is_none());
    }

    #[test]
    fn a_failed_send_keeps_the_draft_and_clears_the_sending_flag() {
        let mut state = logged_in_with_channels(["General"]);
        state.on_key(key(KeyCode::Tab));
        state.on_key(key(KeyCode::Char('h')));
        state.on_key(key(KeyCode::Enter));

        state.on_message_sent("c1".to_string(), Err(ApiError::Server("nope".to_string())));

        let Screen::LoggedIn(logged_in) = &state.screen else {
            panic!("expected LoggedIn");
        };
        assert_eq!(logged_in.compose, "h");
        assert!(!logged_in.sending);
    }

    #[test]
    fn a_successful_send_clears_the_draft_and_reloads_messages() {
        let mut state = logged_in_with_channels(["General"]);
        state.on_key(key(KeyCode::Tab));
        state.on_key(key(KeyCode::Char('h')));
        state.on_key(key(KeyCode::Enter));

        let command = state.on_message_sent("c1".to_string(), Ok(()));

        let Screen::LoggedIn(logged_in) = &state.screen else {
            panic!("expected LoggedIn");
        };
        assert_eq!(logged_in.compose, "");
        assert!(
            matches!(command, Some(Command::LoadMessages { channel_id, .. }) if channel_id == "c1")
        );
    }

    #[test]
    fn a_successful_send_keeps_showing_cached_messages_during_the_reload() {
        // A send used to clear the channel's cached messages up front, which
        // blanked the whole pane until the refetch resolved — the "flash"
        // this test guards against. The cache should stay put; only the
        // fresh page (once `on_messages_loaded` delivers it) replaces it.
        let mut state = logged_in_with_channels(["General"]);
        let existing = Message {
            id: "m1".to_string(),
            kind: "user".to_string(),
            system_event: None,
            body: "<p>already here</p>".to_string(),
            created_at: Utc::now(),
            deleted_at: None,
            author: MessageAuthor {
                id: "u1".to_string(),
                name: "Chris".to_string(),
                preferences: None,
            },
            attachments: Vec::new(),
        };
        state.on_messages_loaded("c1".to_string(), 1, Ok((vec![existing], false)));

        state.on_key(key(KeyCode::Tab));
        state.on_key(key(KeyCode::Char('h')));
        state.on_key(key(KeyCode::Enter));
        state.on_message_sent("c1".to_string(), Ok(()));

        let Screen::LoggedIn(logged_in) = &state.screen else {
            panic!("expected LoggedIn");
        };
        assert_eq!(
            logged_in.messages.get("c1").map(Vec::len),
            Some(1),
            "the cached message should still be there while the reload is in flight"
        );
    }

    #[test]
    fn a_message_event_for_the_selected_channel_reloads_it() {
        let mut state = logged_in_with_channels(["General", "Random"]);
        let command = state.on_realtime_event(RealtimeEvent::MessageCreated {
            channel_id: "c1".to_string(),
        });
        assert!(
            matches!(command, Some(Command::LoadMessages { channel_id, .. }) if channel_id == "c1")
        );
    }

    #[test]
    fn a_message_event_for_a_different_channel_is_ignored() {
        let mut state = logged_in_with_channels(["General", "Random"]);
        let command = state.on_realtime_event(RealtimeEvent::MessageCreated {
            channel_id: "c2".to_string(),
        });
        assert!(command.is_none());
    }

    #[test]
    fn resync_and_channels_changed_reload_channels() {
        let mut state = logged_in_with_channels(["General"]);
        assert!(matches!(
            state.on_realtime_event(RealtimeEvent::Resync),
            Some(Command::LoadChannels)
        ));
        assert!(matches!(
            state.on_realtime_event(RealtimeEvent::ChannelsChanged),
            Some(Command::LoadChannels)
        ));
        assert!(state.on_realtime_event(RealtimeEvent::Ready).is_none());
        assert!(state.on_realtime_event(RealtimeEvent::Other).is_none());
    }

    #[test]
    fn reloading_channels_keeps_the_selection_instead_of_jumping_to_the_first() {
        let mut state = logged_in_with_channels(["General", "Random"]);
        state.on_key(key(KeyCode::Down)); // select "Random" (c2)

        // A channels reload (e.g. from a realtime `channels.changed`) must
        // keep reloading messages for "Random", not silently reset to
        // "General" — this was a real bug caught while adding SSE support.
        let command = state.on_channels_loaded(Ok(vec![
            Channel {
                id: "c1".into(),
                name: "General".into(),
                description: None,
                is_private: false,
                is_archived: false,
                is_favorite: false,
                unread_count: 0,
            },
            Channel {
                id: "c2".into(),
                name: "Random".into(),
                description: None,
                is_private: false,
                is_archived: false,
                is_favorite: false,
                unread_count: 0,
            },
        ]));
        assert!(
            matches!(command, Some(Command::LoadMessages { channel_id, .. }) if channel_id == "c2")
        );
    }

    #[test]
    fn a_stale_messages_response_is_dropped_in_favor_of_the_newer_request() {
        let mut state = logged_in_with_channels(["General"]);

        // Two overlapping requests for the same channel (e.g. a manual
        // refresh followed immediately by an SSE-triggered reload) — the
        // second is issued after the first, so it's the "newer" one.
        let first = state.on_key(key(KeyCode::Char('r')));
        let second = state.on_key(key(KeyCode::Char('r')));
        let (
            Some(Command::LoadMessages { seq: first_seq, .. }),
            Some(Command::LoadMessages {
                seq: second_seq, ..
            }),
        ) = (first, second)
        else {
            panic!("expected two LoadMessages commands");
        };
        assert_ne!(first_seq, second_seq);

        // The newer request's response arrives first and is accepted...
        state.on_messages_loaded("c1".to_string(), second_seq, Ok((vec![], false)));
        // ...then the older, slower response arrives and must be ignored,
        // not overwrite the newer (already-applied) result.
        state.on_messages_loaded(
            "c1".to_string(),
            first_seq,
            Err(ApiError::Server(
                "stale failure, should be ignored".to_string(),
            )),
        );

        let Screen::LoggedIn(logged_in) = &state.screen else {
            panic!("expected LoggedIn");
        };
        assert!(
            logged_in.messages_error.is_none(),
            "the stale error must not surface"
        );
        assert!(logged_in.messages.get("c1").is_some_and(Vec::is_empty));
    }

    #[test]
    fn scrolling_past_the_cached_history_requests_an_older_page() {
        let mut state = logged_in_with_cached_messages(5, true);
        // 5 messages cached; SCROLL_STEP is 10 lines per press, already at
        // or past the message-count proxy on the very first press.
        let command = state.on_key(key(KeyCode::PageUp));
        match command {
            Some(Command::LoadOlderMessages {
                channel_id,
                before_id,
                ..
            }) => {
                assert_eq!(channel_id, "c1");
                assert_eq!(before_id, "m0"); // the oldest cached message
            }
            other => panic!("expected LoadOlderMessages, got {other:?}"),
        }
    }

    #[test]
    fn a_second_page_up_does_not_duplicate_the_in_flight_older_page_request() {
        let mut state = logged_in_with_cached_messages(5, true);
        assert!(state.on_key(key(KeyCode::PageUp)).is_some());
        assert!(
            state.on_key(key(KeyCode::PageUp)).is_none(),
            "already loading — must not fire a second request"
        );
    }

    #[test]
    fn has_more_false_stops_requesting_older_pages() {
        let mut state = logged_in_with_cached_messages(5, false);
        assert!(state.on_key(key(KeyCode::PageUp)).is_none());
    }

    #[test]
    fn older_messages_are_prepended_and_the_scroll_position_is_preserved() {
        let mut state = logged_in_with_cached_messages(5, true);
        state.on_key(key(KeyCode::PageUp)); // triggers the fetch, marks it in flight

        let scroll_before = {
            let Screen::LoggedIn(s) = &state.screen else {
                unreachable!()
            };
            s.message_scroll
        };
        let older = vec![Message {
            id: "m_old".to_string(),
            kind: "user".to_string(),
            system_event: None,
            body: "<p>older</p>".to_string(),
            created_at: Utc::now(),
            deleted_at: None,
            author: MessageAuthor {
                id: "u1".into(),
                name: "Chris".into(),
                preferences: None,
            },
            attachments: Vec::new(),
        }];
        state.on_older_messages_loaded("c1".to_string(), Ok((older, false)));

        let Screen::LoggedIn(logged_in) = &state.screen else {
            panic!("expected LoggedIn");
        };
        let cached = logged_in.messages.get("c1").unwrap();
        assert_eq!(cached.len(), 6);
        assert_eq!(cached[0].id, "m_old", "older messages come first");
        assert_eq!(logged_in.message_scroll, scroll_before + 1);

        // has_more is now known false — a further PageUp must not fetch again.
        assert!(state.on_key(key(KeyCode::PageUp)).is_none());
    }

    #[test]
    fn an_older_page_overlapping_the_cached_boundary_does_not_duplicate_messages() {
        // The server's cursor is exclusive on a millisecond boundary (see
        // docs/api-contract.md) and can hand back a row already cached —
        // this is that exact case: the "older" page's last message is the
        // same id as the existing cache's first (oldest) message.
        let mut state = logged_in_with_cached_messages(5, true);
        state.on_key(key(KeyCode::PageUp));

        let mut older: Vec<Message> = (0..3)
            .map(|i| Message {
                id: format!("old{i}"),
                kind: "user".to_string(),
                system_event: None,
                body: format!("<p>old-{i}</p>"),
                created_at: Utc::now(),
                deleted_at: None,
                author: MessageAuthor {
                    id: "u1".into(),
                    name: "Chris".into(),
                    preferences: None,
                },
                attachments: Vec::new(),
            })
            .collect();
        older.push(Message {
            id: "m0".to_string(), // duplicates the existing cache's oldest message
            kind: "user".to_string(),
            system_event: None,
            body: "<p>msg-0</p>".to_string(),
            created_at: Utc::now(),
            deleted_at: None,
            author: MessageAuthor {
                id: "u1".into(),
                name: "Chris".into(),
                preferences: None,
            },
            attachments: Vec::new(),
        });
        state.on_older_messages_loaded("c1".to_string(), Ok((older, false)));

        let Screen::LoggedIn(logged_in) = &state.screen else {
            panic!("expected LoggedIn");
        };
        let cached = logged_in.messages.get("c1").unwrap();
        // 3 genuinely new + 5 original = 8, not 9 — the duplicate "m0" was dropped.
        assert_eq!(cached.len(), 8);
        assert_eq!(cached.iter().filter(|m| m.id == "m0").count(), 1);
    }

    /// A logged-in state with N channels named `c1..cN`, ready for
    /// channel-navigation or composer tests.
    fn logged_in_with_channels<const N: usize>(names: [&str; N]) -> AppState {
        let mut state = AppState::resuming();
        state.on_resume_finished(Some(User {
            id: "u1".into(),
            name: "Chris".into(),
            email: "chris@example.com".into(),
            approval_status: "approved".into(),
        }));
        let channels = names
            .into_iter()
            .enumerate()
            .map(|(i, name)| Channel {
                id: format!("c{}", i + 1),
                name: name.to_string(),
                description: None,
                is_private: false,
                is_archived: false,
                is_favorite: false,
                unread_count: 0,
            })
            .collect();
        state.on_channels_loaded(Ok(channels));
        state
    }

    /// A logged-in state with one channel ("c1") already holding `count`
    /// cached messages (ids `m0..m{count}`, oldest first), as if an initial
    /// load already completed.
    fn logged_in_with_cached_messages(count: usize, has_more: bool) -> AppState {
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
        let messages: Vec<Message> = (0..count)
            .map(|i| Message {
                id: format!("m{i}"),
                kind: "user".to_string(),
                system_event: None,
                body: format!("<p>msg-{i}</p>"),
                created_at: Utc::now(),
                deleted_at: None,
                author: MessageAuthor {
                    id: "u1".into(),
                    name: "Chris".into(),
                    preferences: None,
                },
                attachments: Vec::new(),
            })
            .collect();
        state.on_messages_loaded("c1".to_string(), seq, Ok((messages, has_more)));
        state
    }
}
