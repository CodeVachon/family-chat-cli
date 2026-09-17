//! Ratatui layout, rendering, the event loop, and terminal lifecycle (#13/#24/#27).

mod layout;
mod widgets;

use std::io;
use std::sync::Arc;

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures_util::StreamExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::sync::mpsc;

use crate::api::{ApiClient, RealtimeStream};
use crate::app::{AppState, Command, Event};
use crate::auth::{CredentialStore, login};

type Term = Terminal<CrosstermBackend<io::Stdout>>;

pub async fn run(client: ApiClient, store: Box<dyn CredentialStore>) -> anyhow::Result<()> {
    let mut terminal = init_terminal()?;
    // However the app loop exits — quit key, error, or panic unwinding through
    // here — the terminal must come back. `restore_terminal` runs on every
    // path, including a panic (see `main`'s panic hook for the case where
    // unwinding doesn't reach this far).
    let result = run_app(&mut terminal, client, store).await;
    restore_terminal()?;
    result
}

fn init_terminal() -> anyhow::Result<Term> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

pub fn restore_terminal() -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

async fn run_app(
    terminal: &mut Term,
    client: ApiClient,
    store: Box<dyn CredentialStore>,
) -> anyhow::Result<()> {
    let store: Arc<dyn CredentialStore> = Arc::from(store);
    let mut state = AppState::resuming();
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let mut crossterm_events = EventStream::new();
    // Started once per login session (see the `LoadChannels` arm below) and
    // torn down on logout — otherwise a dead/rejected token would leave a
    // stream perpetually retrying against a session that's gone.
    let mut realtime_task: Option<tokio::task::JoinHandle<()>> = None;

    spawn_resume(client.clone(), store.clone(), tx.clone());
    terminal.draw(|frame| widgets::render(frame, &state))?;

    loop {
        let command = tokio::select! {
            maybe_event = crossterm_events.next() => match maybe_event {
                Some(Ok(CrosstermEvent::Key(key))) if key.kind == KeyEventKind::Press => state.on_key(key),
                Some(Ok(_)) => None,
                Some(Err(error)) => {
                    tracing::error!(%error, "terminal event stream error");
                    None
                }
                None => Some(Command::Quit),
            },
            event = rx.recv() => match event {
                Some(Event::ResumeFinished(user)) => state.on_resume_finished(user),
                Some(Event::LoginFinished(result)) => state.on_login_finished(result),
                Some(Event::ChannelsLoaded(result)) => state.on_channels_loaded(result),
                Some(Event::MessagesLoaded { channel_id, seq, result }) => {
                    state.on_messages_loaded(channel_id, seq, result);
                    None
                }
                Some(Event::OlderMessagesLoaded { channel_id, result }) => {
                    state.on_older_messages_loaded(channel_id, result);
                    None
                }
                Some(Event::MessageSent { channel_id, result }) => state.on_message_sent(channel_id, result),
                Some(Event::MembersLoaded { channel_id, result }) => {
                    state.on_members_loaded(channel_id, result);
                    None
                }
                Some(Event::Realtime(event)) => state.on_realtime_event(event),
                None => None,
            },
        };

        if let Some(command) = command {
            match command {
                Command::Quit => break,
                Command::SubmitLogin { email, password } => {
                    state.on_login_submitted();
                    spawn_login(client.clone(), store.clone(), tx.clone(), email, password);
                }
                Command::LoadChannels => {
                    spawn_load_channels(client.clone(), tx.clone());
                    // `LoadChannels` also fires from realtime resync/change
                    // events (see app::state::on_realtime_event) — only
                    // start the stream itself the first time, on the
                    // login/resume transition into this screen.
                    if realtime_task.is_none() {
                        realtime_task = Some(spawn_realtime(client.clone(), tx.clone()));
                    }
                }
                Command::LoadMessages { channel_id, seq } => {
                    tracing::info!(channel_id = %channel_id, seq, "requesting latest messages");
                    state.on_messages_loading();
                    // Every LoadMessages is for the channel currently being
                    // viewed (see `app::state::request_messages`'s doc
                    // comment) — that's exactly when the server should also
                    // be told the user has caught up (#33). Fire-and-forget:
                    // the unread badge is already cleared locally, and a
                    // failure here just leaves the server's own count
                    // briefly stale, logged for diagnosis like every other
                    // command (see `log_if_err`).
                    spawn_mark_channel_read(client.clone(), channel_id.clone());
                    // Same reasoning as mark_channel_read: every LoadMessages
                    // means "the user is looking at this channel," which is
                    // also a fine moment to keep its member list current
                    // (#50). Refetched every time rather than cached-once, so
                    // the manual refresh key ('r') also refreshes users.
                    spawn_load_members(client.clone(), tx.clone(), channel_id.clone());
                    spawn_load_messages(client.clone(), tx.clone(), channel_id, seq);
                }
                Command::LoadOlderMessages {
                    channel_id,
                    before_id,
                    before_created_at,
                } => {
                    spawn_load_older_messages(
                        client.clone(),
                        tx.clone(),
                        channel_id,
                        before_id,
                        before_created_at,
                    );
                }
                Command::SendMessage { channel_id, body } => {
                    spawn_send_message(client.clone(), tx.clone(), channel_id, body);
                }
                Command::Logout => {
                    state.on_logout();
                    if let Some(task) = realtime_task.take() {
                        task.abort();
                    }
                    spawn_logout(client.clone(), store.clone());
                }
            }
        }

        if state.should_quit {
            break;
        }

        terminal.draw(|frame| widgets::render(frame, &state))?;
    }

    Ok(())
}

fn spawn_resume(
    client: ApiClient,
    store: Arc<dyn CredentialStore>,
    tx: mpsc::UnboundedSender<Event>,
) {
    tokio::spawn(async move {
        let user = login::resume(&client, &*store).await;
        let _ = tx.send(Event::ResumeFinished(user));
    });
}

fn spawn_login(
    client: ApiClient,
    store: Arc<dyn CredentialStore>,
    tx: mpsc::UnboundedSender<Event>,
    email: String,
    password: String,
) {
    tokio::spawn(async move {
        let result = login::sign_in(&client, &*store, &email, &password)
            .await
            .map(|session| session.user);
        log_if_err("sign in", &result);
        let _ = tx.send(Event::LoginFinished(result));
    });
}

fn spawn_load_channels(client: ApiClient, tx: mpsc::UnboundedSender<Event>) {
    tokio::spawn(async move {
        let result = client
            .list_channels()
            .await
            .map(|response| response.channels);
        log_if_err("load channels", &result);
        let _ = tx.send(Event::ChannelsLoaded(result));
    });
}

fn spawn_load_messages(
    client: ApiClient,
    tx: mpsc::UnboundedSender<Event>,
    channel_id: String,
    seq: u64,
) {
    tokio::spawn(async move {
        let result = client
            .channel_messages(&channel_id, None)
            .await
            .map(|response| (response.messages, response.has_more));
        log_if_err("load messages", &result);
        log_load_result("load messages", &channel_id, &result);
        let _ = tx.send(Event::MessagesLoaded {
            channel_id,
            seq,
            result,
        });
    });
}

fn spawn_load_older_messages(
    client: ApiClient,
    tx: mpsc::UnboundedSender<Event>,
    channel_id: String,
    before_id: String,
    before_created_at: chrono::DateTime<chrono::Utc>,
) {
    tokio::spawn(async move {
        let result = client
            .channel_messages(&channel_id, Some((&before_id, before_created_at)))
            .await
            .map(|response| (response.messages, response.has_more));
        log_if_err("load older messages", &result);
        log_load_result("load older messages", &channel_id, &result);
        let _ = tx.send(Event::OlderMessagesLoaded { channel_id, result });
    });
}

fn spawn_send_message(
    client: ApiClient,
    tx: mpsc::UnboundedSender<Event>,
    channel_id: String,
    body: String,
) {
    tokio::spawn(async move {
        let html = crate::text::html::plain_text_to_html(&body);
        let result = client.send_message(&channel_id, &html).await;
        log_if_err("send message", &result);
        let _ = tx.send(Event::MessageSent { channel_id, result });
    });
}

/// Fire-and-forget (#33) — the local unread badge is already cleared
/// synchronously by `app::state::request_messages`; this just tells the
/// server so every other client sees the same cleared count too. No
/// `Event` is sent back: nothing in the UI needs to react to this
/// succeeding or failing beyond what's already logged.
fn spawn_mark_channel_read(client: ApiClient, channel_id: String) {
    tokio::spawn(async move {
        let result = client.mark_channel_read(&channel_id).await;
        log_if_err("mark channel read", &result);
    });
}

fn spawn_load_members(client: ApiClient, tx: mpsc::UnboundedSender<Event>, channel_id: String) {
    tokio::spawn(async move {
        let result = client
            .channel_members(&channel_id)
            .await
            .map(|response| response.members);
        log_if_err("load users", &result);
        let _ = tx.send(Event::MembersLoaded { channel_id, result });
    });
}

fn spawn_realtime(
    client: ApiClient,
    tx: mpsc::UnboundedSender<Event>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut stream = match RealtimeStream::connect(&client) {
            Ok(stream) => stream,
            Err(error) => {
                tracing::error!(%error, "could not start the realtime stream");
                return;
            }
        };
        while let Some(event) = stream.next().await {
            tracing::info!(?event, "realtime event received");
            if tx.send(Event::Realtime(event)).is_err() {
                return; // the main loop is gone; nothing left to deliver to.
            }
        }
        tracing::info!("realtime stream ended (won't retry — see RealtimeStream::next)");
    })
}

fn spawn_logout(client: ApiClient, store: Arc<dyn CredentialStore>) {
    tokio::spawn(async move {
        let result = login::sign_out(&client, &*store).await;
        log_if_err("sign out", &result);
    });
}

/// Every API-backed command logs its own failure here (to the log file, never
/// the terminal — see `main::init_logging`) so a "server error" in the status
/// line is diagnosable afterward without re-running with `RUST_LOG=debug`.
/// This is what was missing when #31's send-message 500 first showed up.
fn log_if_err<T>(what: &'static str, result: &Result<T, crate::api::ApiError>) {
    if let Err(error) = result {
        tracing::warn!(what, %error, "command failed");
    }
}

/// Logs a successful message load's shape (count, newest timestamp) at INFO
/// — added specifically to diagnose "the TUI shows older messages than the
/// server has": without this, a successful-but-somehow-wrong load left no
/// trace to compare against a direct API query.
fn log_load_result(
    what: &'static str,
    channel_id: &str,
    result: &Result<(Vec<crate::api::types::Message>, bool), crate::api::ApiError>,
) {
    if let Ok((messages, has_more)) = result {
        let newest = messages.last().map(|m| m.created_at.to_rfc3339());
        tracing::info!(
            what,
            channel_id,
            count = messages.len(),
            has_more,
            newest = newest.as_deref(),
            "load succeeded"
        );
    }
}
