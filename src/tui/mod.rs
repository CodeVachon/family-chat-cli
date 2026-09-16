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

use crate::api::ApiClient;
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
                Some(Event::MessagesLoaded { channel_id, result }) => {
                    state.on_messages_loaded(channel_id, result);
                    None
                }
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
                Command::LoadChannels => spawn_load_channels(client.clone(), tx.clone()),
                Command::LoadMessages { channel_id } => {
                    state.on_messages_loading();
                    spawn_load_messages(client.clone(), tx.clone(), channel_id);
                }
                Command::Logout => {
                    state.on_logout();
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
        let _ = tx.send(Event::LoginFinished(result));
    });
}

fn spawn_load_channels(client: ApiClient, tx: mpsc::UnboundedSender<Event>) {
    tokio::spawn(async move {
        let result = client
            .list_channels()
            .await
            .map(|response| response.channels);
        let _ = tx.send(Event::ChannelsLoaded(result));
    });
}

fn spawn_load_messages(client: ApiClient, tx: mpsc::UnboundedSender<Event>, channel_id: String) {
    tokio::spawn(async move {
        let result = client
            .channel_messages(&channel_id)
            .await
            .map(|response| response.messages);
        let _ = tx.send(Event::MessagesLoaded { channel_id, result });
    });
}

fn spawn_logout(client: ApiClient, store: Arc<dyn CredentialStore>) {
    tokio::spawn(async move {
        login::sign_out(&client, &*store).await;
    });
}
