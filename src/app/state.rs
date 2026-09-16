//! State + reducer-style transitions (#12). No IO, so directly unit-testable (#42).

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::api::ApiError;
use crate::api::types::{Channel, Message, User};

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
    pub status: Option<String>,
    pub focus: LoggedInFocus,
    pub compose: String,
    pub sending: bool,
}

impl LoggedInState {
    fn new(user: User) -> Self {
        Self {
            user,
            channels: None,
            selected: 0,
            messages: HashMap::new(),
            loading_messages: false,
            status: None,
            focus: LoggedInFocus::Channels,
            compose: String::new(),
            sending: false,
        }
    }

    pub fn selected_channel(&self) -> Option<&Channel> {
        self.channels.as_ref().and_then(|c| c.get(self.selected))
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
    LoggedIn(LoggedInState),
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

    fn on_logged_in_key(
        state: &mut LoggedInState,
        key: KeyEvent,
        should_quit: &mut bool,
    ) -> Option<Command> {
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
                let channel = state.selected_channel()?;
                Some(Command::LoadMessages {
                    channel_id: channel.id.clone(),
                })
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected > 0 {
                    state.selected -= 1;
                    return Self::load_selected(state);
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = state.channels.as_ref().map_or(0, Vec::len);
                if state.selected + 1 < len {
                    state.selected += 1;
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
                None
            }
            KeyCode::Backspace => {
                state.compose.pop();
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

    fn load_selected(state: &LoggedInState) -> Option<Command> {
        let channel = state.selected_channel()?;
        if state.messages.contains_key(&channel.id) {
            return None;
        }
        Some(Command::LoadMessages {
            channel_id: channel.id.clone(),
        })
    }

    pub fn on_resume_finished(&mut self, user: Option<User>) -> Option<Command> {
        match user {
            Some(user) => {
                self.screen = Screen::LoggedIn(LoggedInState::new(user));
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
                self.screen = Screen::LoggedIn(LoggedInState::new(user));
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
                let first = channels.first().map(|c| c.id.clone());
                state.channels = Some(channels);
                first.map(|channel_id| Command::LoadMessages { channel_id })
            }
            Err(error) => {
                state.status = Some(format!("Couldn't load channels: {error}"));
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
        result: Result<Vec<Message>, ApiError>,
    ) {
        let Screen::LoggedIn(state) = &mut self.screen else {
            return;
        };
        state.loading_messages = false;
        match result {
            Ok(messages) => {
                state.messages.insert(channel_id, messages);
            }
            Err(error) => {
                state.status = Some(format!("Couldn't load messages: {error}"));
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
                // Force a reload rather than splicing the new message in
                // locally: the send response doesn't carry the decorated
                // shape (author/reactions/mentions) GET returns, and a
                // refetch is simple and correct without SSE to reconcile
                // against yet (see #57).
                state.messages.remove(&channel_id);
                Some(Command::LoadMessages { channel_id })
            }
            Err(error) => {
                state.status = Some(format!("Couldn't send message: {error}"));
                None
            }
        }
    }

    pub fn on_logout(&mut self) {
        self.screen = Screen::LoggedOut(LoginForm::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            Some(Command::LoadMessages { channel_id }) => assert_eq!(channel_id, "c2"),
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
            matches!(command, Some(Command::LoadMessages { channel_id }) if channel_id == "c1")
        );
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
}
