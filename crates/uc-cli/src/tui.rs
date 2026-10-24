//! The TUI of `uc`: `uc` without a subcommand, or `uc tui`.
//!
//! The screen:
//!   - top: tab bar (Catalogs / Schemas / Tables / Volumes / …)
//!   - left: list of items in the current tab
//!   - right: pretty-JSON detail of the selected item; without one, the
//!     logo ([`logo`])
//!   - bottom: status line (server URL, last error, key hints)
//!
//! Networking is async: each refetch spawns a tokio task that sends results
//! back via an mpsc channel. The main loop `tokio::select!`s between the
//! crossterm event stream and that channel, so the UI never blocks on HTTP.
//!
//! ## Modals
//!
//! One `Modal` at most is over the main view, and `Esc` closes it: Help,
//! Confirm (a yes/no question before a delete), TextInput (a line of text,
//! such as the server URL) and Secret (a new token, shown once).
//!
//! ## Persistence
//!
//! The tab and the scope are kept in
//! `${XDG_STATE_HOME:-~/.local/state}/uc/state.json` on quit, and restored
//! at the next start.
//!
//! ## Sign-in
//!
//! Without a given token, the TUI sends the token of the sign-in to its
//! server that `uc auth login` keeps ([`crate::sign_in`]): from the next
//! fetch after a new sign-in, and for the server that `s` sets. After a
//! 401, the pane says how to sign in.

mod logo;

use std::borrow::Cow;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, TimeZone};
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::{Stream, StreamExt};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use uc_client::error::Error as ClientError;
use uc_client::models::credentials::v1::{AwsIamRoleRequest, CredentialPurpose};
use uc_client::models::scim::v1::ScimEmail;
use uc_client::models::volumes::v1::VolumeType;
use uc_client::{Client, Contents};
use uc_common::settings::Settings;
use uc_common::{API_BASE, ErrorCode};

use crate::args::Global;
use crate::commands::call_error;
use crate::config::{self, TokenSource};
use crate::output::models::{self, Model, with_empty};
use crate::sign_in::{self, SignIns};

// Layout breakpoints for [`App::render`]. Below `MIN_*`, the screen only
// says that the window is too small; below `COMPACT_*`, the tab titles and
// the key hints are short; below `STACKED_*`, the list is above the detail.
const MIN_WIDTH: u16 = 40;
const MIN_HEIGHT: u16 = 8;
const COMPACT_WIDTH: u16 = 100;
const STACKED_WIDTH: u16 = 60;
/// How far PgUp/PgDn moves the detail-pane scroll position per keystroke.
const DETAIL_SCROLL_STEP: u16 = 10;
/// The expiry that the TUI offers for a new token, in days.
const TOKEN_DAYS: u32 = 30;
/// How long the text input's cursor `_` shows, and then hides: it blinks.
const BLINK: Duration = Duration::from_millis(500);

/// Run the TUI, for `uc` without a subcommand or `uc tui`, against the
/// server of the `uc` commands ([`config`]).
pub async fn run(global: &Global, settings: &Settings) -> anyhow::Result<()> {
    let origin = config::origin(&config::server(settings, global.server.as_deref()))?;
    let server_url = format!("{origin}{API_BASE}/");
    let sign_ins = SignIns::of_user();
    let token = config::token(
        settings,
        global.auth_token.as_deref(),
        &origin,
        sign_ins.as_ref(),
    );
    let client = config::client(&server_url, token.as_ref().map(|(token, _)| token.as_str()))?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(client, server_url, state_path()).with_sign_ins(
        sign_ins,
        token,
        sign_in::login_command(global),
    );
    app.true_color = crossterm::style::available_color_count() == u16::MAX;
    app.restore_persisted_state();

    let result = app.run(&mut terminal, EventStream::new()).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

/// One per browsable resource, in the order of the tab bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(usize)]
enum Tab {
    Catalogs = 0,
    Schemas,
    Tables,
    Volumes,
    Functions,
    Models,
    Credentials,
    ExternalLocations,
    Namespaces,
    ScimUsers,
    Tokens,
}

impl Tab {
    const ALL: [Tab; 11] = [
        Tab::Catalogs,
        Tab::Schemas,
        Tab::Tables,
        Tab::Volumes,
        Tab::Functions,
        Tab::Models,
        Tab::Credentials,
        Tab::ExternalLocations,
        Tab::Namespaces,
        Tab::ScimUsers,
        Tab::Tokens,
    ];

    fn title(self) -> &'static str {
        match self {
            Tab::Catalogs => "Catalogs",
            Tab::Schemas => "Schemas",
            Tab::Tables => "Tables",
            Tab::Volumes => "Volumes",
            Tab::Functions => "Functions",
            Tab::Models => "Models",
            Tab::Credentials => "Credentials",
            Tab::ExternalLocations => "ExternalLocs",
            Tab::Namespaces => "Namespaces",
            Tab::ScimUsers => "ScimUsers",
            Tab::Tokens => "Tokens",
        }
    }

    /// 3-char title used in compact mode, so the tab bar takes fewer lines.
    /// It has no spaces: [`tab_lines`] adds them, and the divider.
    fn short_title(self) -> &'static str {
        match self {
            Tab::Catalogs => "Cat",
            Tab::Schemas => "Sch",
            Tab::Tables => "Tbl",
            Tab::Volumes => "Vol",
            Tab::Functions => "Fn",
            Tab::Models => "Mdl",
            Tab::Credentials => "Crd",
            Tab::ExternalLocations => "ExL",
            Tab::Namespaces => "NS",
            Tab::ScimUsers => "Usr",
            Tab::Tokens => "Tok",
        }
    }

    /// What an object of the tab is called, when the server deletes it only
    /// while it holds nothing, unless the call sets `force`.
    fn holder(self) -> Option<&'static str> {
        match self {
            Tab::Catalogs => Some("catalog"),
            Tab::Schemas => Some("schema"),
            Tab::Models => Some("model"),
            _ => None,
        }
    }

    fn index(self) -> usize {
        // The discriminant is the slot in [`Self::ALL`] and [`App::panes`].
        self as usize
    }

    fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    fn prev(self) -> Self {
        let n = Self::ALL.len();
        Self::ALL[(self.index() + n - 1) % n]
    }

    /// Tabs that browse resources scoped to a schema (catalog.schema). These
    /// rely on `App::current_catalog` + `App::current_schema` filters.
    fn needs_schema_scope(self) -> bool {
        matches!(self, Tab::Tables | Tab::Volumes | Tab::Functions)
    }

    /// Tabs scoped to a single catalog (just the catalog filter).
    fn needs_catalog_scope(self) -> bool {
        matches!(self, Tab::Schemas)
    }
}

/// One row in the list pane: a short label + the source JSON for the detail
/// pane + the identifier the delete handler needs.
#[derive(Clone)]
struct Row {
    label: String,
    detail: serde_json::Value,
    /// Identifier passed to the delete API for this row (catalog name, full
    /// name, id, etc.). `None` means delete isn't available for this row.
    delete_id: Option<String>,
}

#[derive(Default)]
struct Pane {
    rows: Vec<Row>,
    list_state: ListState,
    loading: bool,
    error: Option<String>,
    /// Vertical scroll offset for the detail pane (lines).
    detail_scroll: u16,
}

impl Pane {
    fn set_rows(&mut self, rows: Vec<Row>) {
        self.rows = rows;
        self.error = None;
        self.loading = false;
        self.detail_scroll = 0;
        self.list_state
            .select(if self.rows.is_empty() { None } else { Some(0) });
    }

    fn set_error(&mut self, msg: String) {
        self.rows.clear();
        self.error = Some(msg);
        self.loading = false;
        self.list_state.select(None);
    }

    fn select_next(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let i = self
            .list_state
            .selected()
            .map(|i| (i + 1) % self.rows.len())
            .unwrap_or(0);
        self.list_state.select(Some(i));
        self.detail_scroll = 0;
    }

    fn select_prev(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let i = self
            .list_state
            .selected()
            .map(|i| if i == 0 { self.rows.len() - 1 } else { i - 1 })
            .unwrap_or(0);
        self.list_state.select(Some(i));
        self.detail_scroll = 0;
    }

    fn selected(&self) -> Option<&Row> {
        self.list_state.selected().and_then(|i| self.rows.get(i))
    }
}

/// Result of a background fetch, returned via the App's channel.
struct FetchResult {
    tab: Tab,
    /// The tab's fetch generation when the fetch started. A result of an
    /// older generation is dropped, so that it cannot overwrite a newer one.
    generation: u64,
    rows: Result<Vec<Row>, Failure>,
}

/// Result of a background mutation: the tab to refresh, and the outcome.
struct MutationResult {
    tab: Tab,
    outcome: Result<MutationOutcome, Failure>,
}

enum MutationOutcome {
    /// Plain status-line message.
    Message(String),
    /// A delete that the server refused because the catalog, schema or
    /// model holds something: the question whether to delete it all.
    NotEmpty {
        id: String,
        /// The server's reason.
        reason: String,
        /// What it holds, or why the count failed.
        contents: Result<Contents, String>,
    },
    /// Surface in a one-shot secret modal.
    Secret {
        title: String,
        secret: String,
        hint: String,
    },
}

/// A failed fetch or change: its text, and whether the server asked for a
/// sign-in (401).
#[derive(Debug)]
struct Failure {
    text: String,
    needs_sign_in: bool,
}

impl From<ClientError> for Failure {
    fn from(e: ClientError) -> Self {
        Self {
            needs_sign_in: e.status() == Some(401),
            text: message(&e),
        }
    }
}

/// What failed in a call, as the TUI shows it ([`call_error`]).
fn message(e: &ClientError) -> String {
    call_error(e).message().to_string()
}

impl From<String> for Failure {
    fn from(text: String) -> Self {
        Self {
            text,
            needs_sign_in: false,
        }
    }
}

impl From<&str> for Failure {
    fn from(text: &str) -> Self {
        text.to_string().into()
    }
}

/// Overlay state. At most one modal is active; `Esc` always closes it.
enum Modal {
    None,
    Help,
    /// Yes/no confirmation. On `y`, runs `action`.
    Confirm {
        prompt: String,
        action: PendingAction,
    },
    /// Single-line text input. On Enter, calls `submit` with the value.
    TextInput {
        prompt: String,
        value: String,
        submit: TextSubmit,
    },
    /// A value that the server returns only once, such as the token of a
    /// new PAT. Only Enter or Esc closes it, so that the user can copy it.
    Secret {
        title: String,
        secret: String,
        hint: String,
    },
}

/// What the confirm modal will execute when accepted.
#[derive(Clone)]
enum PendingAction {
    /// Delete `id` on `tab`; with `force`, also all it holds.
    Delete { tab: Tab, id: String, force: bool },
    /// After the server refused to delete `id` because it holds something:
    /// ask whether the user is sure to delete it with all it holds.
    Force {
        tab: Tab,
        id: String,
        contents: Option<Contents>,
    },
}

impl PendingAction {
    /// The title of its question, and the key that answers yes. `n` and
    /// Esc cancel.
    fn question(&self) -> (&'static str, &'static str) {
        match self {
            PendingAction::Delete { force: false, .. } => (" Confirm ", "y = confirm"),
            PendingAction::Force { .. } => (" Not empty ", "f = also delete all it holds"),
            PendingAction::Delete { force: true, .. } => {
                (" Are you sure? ", "y = delete everything")
            }
        }
    }

    /// Whether `key` answers yes: `y` or Enter for a delete, `f` for the
    /// step to the delete of all it holds, and only `y` for that delete.
    fn accepts(&self, key: KeyCode) -> bool {
        match self {
            PendingAction::Delete { force: false, .. } => {
                matches!(key, KeyCode::Char('y' | 'Y') | KeyCode::Enter)
            }
            PendingAction::Force { .. } => matches!(key, KeyCode::Char('f' | 'F')),
            PendingAction::Delete { force: true, .. } => matches!(key, KeyCode::Char('y' | 'Y')),
        }
    }
}

/// What the text-input modal does on submit.
#[derive(Clone)]
enum TextSubmit {
    /// Rebuild the HTTP client with the new server URL.
    SetServer,
    /// Create a row on the given tab; [`create_prompt`] gives the format of
    /// the input.
    CreateOnTab(Tab),
    /// Create the personal access token of this name: the input is its
    /// expiry ([`token_expiry`]).
    TokenExpiry(String),
}

struct App {
    client: Arc<Client>,
    server_url: String,
    tab: Tab,
    panes: [Pane; 11],
    /// Bumped on each fetch dispatch; see [`FetchResult::generation`].
    fetch_gen: [u64; 11],
    current_catalog: Option<String>,
    current_schema: Option<String>,
    /// One-line status message shown at the bottom.
    status_msg: Option<String>,
    /// The text of the last failed change: red while it is the status
    /// message.
    failure: Option<String>,
    modal: Modal,
    /// Whether the text input shows its cursor now ([`BLINK`]).
    cursor_shown: bool,
    should_quit: bool,
    tx_fetch: mpsc::UnboundedSender<FetchResult>,
    rx_fetch: mpsc::UnboundedReceiver<FetchResult>,
    tx_mut: mpsc::UnboundedSender<MutationResult>,
    rx_mut: mpsc::UnboundedReceiver<MutationResult>,
    /// Where the tab and scope are kept between runs; see [`state_path`].
    state_file: Option<PathBuf>,
    /// The sign-ins of `uc auth login`, which the TUI follows
    /// ([`App::follow_sign_in`]); none in tests.
    sign_ins: Option<SignIns>,
    /// The client's token and where it came from; none without a token.
    token: Option<(String, TokenSource)>,
    /// The command that signs in, for a 401 ([`sign_in::login_command`]).
    login: String,
    /// Whether the terminal shows 24-bit colors. Without them, the screen
    /// takes the nearest of xterm's 256 colors ([`xterm_256`]).
    true_color: bool,
}

impl App {
    fn new(client: Client, server_url: String, state_file: Option<PathBuf>) -> Self {
        let (tx_fetch, rx_fetch) = mpsc::unbounded_channel();
        let (tx_mut, rx_mut) = mpsc::unbounded_channel();
        Self {
            client: Arc::new(client),
            server_url,
            tab: Tab::Catalogs,
            panes: Default::default(),
            fetch_gen: [0; 11],
            current_catalog: None,
            current_schema: None,
            status_msg: None,
            failure: None,
            modal: Modal::None,
            cursor_shown: true,
            should_quit: false,
            tx_fetch,
            rx_fetch,
            tx_mut,
            rx_mut,
            state_file,
            sign_ins: None,
            token: None,
            login: "uc auth login".to_string(),
            true_color: true,
        }
    }

    /// Follow the sign-ins in `sign_ins`; the client sends `token`, and
    /// `login` signs in.
    fn with_sign_ins(
        mut self,
        sign_ins: Option<SignIns>,
        token: Option<(String, TokenSource)>,
        login: String,
    ) -> Self {
        self.sign_ins = sign_ins;
        self.token = token;
        self.login = login;
        self
    }

    fn pane(&self) -> &Pane {
        &self.panes[self.tab.index()]
    }
    fn pane_mut(&mut self) -> &mut Pane {
        &mut self.panes[self.tab.index()]
    }

    /// Draw on `terminal` and handle `events` until the user quits.
    async fn run<B: Backend>(
        mut self,
        terminal: &mut Terminal<B>,
        mut events: impl Stream<Item = io::Result<Event>> + Unpin,
    ) -> anyhow::Result<()> {
        self.refresh_current();
        // The text input's cursor blinks while no key comes.
        let mut blink = tokio::time::interval(BLINK);
        blink.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            terminal.draw(|f| self.render(f))?;
            if self.should_quit {
                self.persist_state();
                break;
            }

            let typing = matches!(self.modal, Modal::TextInput { .. });
            tokio::select! {
                Some(ev) = events.next() => {
                    if let Ok(ev) = ev {
                        self.handle_event(ev);
                    }
                    // A key shows the cursor at once, for a full blink.
                    self.cursor_shown = true;
                    blink.reset();
                }
                Some(fr) = self.rx_fetch.recv() => self.apply_fetch_result(fr),
                Some(mr) = self.rx_mut.recv() => self.apply_mutation_result(mr),
                _ = blink.tick(), if typing => self.cursor_shown = !self.cursor_shown,
            }
        }
        Ok(())
    }

    fn handle_event(&mut self, ev: Event) {
        let Event::Key(k) = ev else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }

        // ctrl-c always quits, even with a modal open.
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        // Modal handling preempts the main keymap.
        match &mut self.modal {
            Modal::Help => {
                self.modal = Modal::None;
                return;
            }
            Modal::Secret { .. } => {
                // A stray key does not close it: the secret shows only once.
                if matches!(k.code, KeyCode::Enter | KeyCode::Esc) {
                    self.modal = Modal::None;
                }
                return;
            }
            Modal::Confirm { action, .. } => {
                if action.accepts(k.code) {
                    let action = action.clone();
                    self.modal = Modal::None;
                    self.run_action(action);
                } else if matches!(k.code, KeyCode::Char('n' | 'N') | KeyCode::Esc) {
                    self.modal = Modal::None;
                }
                return;
            }
            Modal::TextInput { value, submit, .. } => {
                match k.code {
                    KeyCode::Enter => {
                        let v = std::mem::take(value);
                        let submit = submit.clone();
                        self.modal = Modal::None;
                        self.run_text_submit(submit, v);
                    }
                    KeyCode::Esc => self.modal = Modal::None,
                    KeyCode::Backspace => {
                        value.pop();
                    }
                    KeyCode::Char(c) => value.push(c),
                    _ => {}
                }
                return;
            }
            Modal::None => {}
        }

        let shift = k.modifiers.contains(KeyModifiers::SHIFT);
        match k.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('?') => self.modal = Modal::Help,
            KeyCode::Char('r') => self.refresh_current(),
            KeyCode::Char('s') => {
                self.modal = Modal::TextInput {
                    prompt: "Server URL".to_string(),
                    value: self.server_url.clone(),
                    submit: TextSubmit::SetServer,
                };
            }
            KeyCode::Char('d') => self.confirm_delete(),
            KeyCode::Char('c') => self.prompt_create(),
            KeyCode::Tab | KeyCode::Right => {
                self.tab = self.tab.next();
                self.status_msg = None;
                self.refresh_current();
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.tab = self.tab.prev();
                self.status_msg = None;
                self.refresh_current();
            }
            // Shift-↓ and Shift-↑ scroll the detail by a line, also on
            // keyboards without PgDn and PgUp.
            KeyCode::Down if shift => {
                self.pane_mut().detail_scroll = self.pane().detail_scroll.saturating_add(1);
            }
            KeyCode::Up if shift => {
                self.pane_mut().detail_scroll = self.pane().detail_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => self.pane_mut().select_next(),
            KeyCode::Up | KeyCode::Char('k') => self.pane_mut().select_prev(),
            KeyCode::PageDown | KeyCode::Char(' ') => {
                self.pane_mut().detail_scroll =
                    self.pane().detail_scroll.saturating_add(DETAIL_SCROLL_STEP);
            }
            KeyCode::PageUp => {
                self.pane_mut().detail_scroll =
                    self.pane().detail_scroll.saturating_sub(DETAIL_SCROLL_STEP);
            }
            KeyCode::Enter => self.drill_in(),
            KeyCode::Backspace => self.drill_out(),
            _ => {}
        }
    }

    /// Enter on a catalog or schema row sets the filter and jumps to the
    /// child tab. Elsewhere, it's a no-op.
    fn drill_in(&mut self) {
        let target = self.pane().selected().and_then(|row| match self.tab {
            Tab::Catalogs => row
                .detail
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            Tab::Schemas => row
                .detail
                .get("full_name")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            _ => None,
        });
        let Some(target) = target else { return };

        match self.tab {
            Tab::Catalogs => {
                self.current_catalog = Some(target.clone());
                self.current_schema = None;
                self.status_msg = Some(format!("scope: catalog={target}"));
                self.tab = Tab::Schemas;
                self.refresh_current();
            }
            Tab::Schemas => {
                if let Some((cat, sch)) = target.split_once('.') {
                    self.current_catalog = Some(cat.to_string());
                    self.current_schema = Some(sch.to_string());
                    self.status_msg = Some(format!("scope: schema={cat}.{sch}"));
                    self.tab = Tab::Tables;
                    self.refresh_current();
                }
            }
            _ => {}
        }
    }

    /// Backspace clears the most-specific filter and refetches.
    fn drill_out(&mut self) {
        if self.current_schema.is_some() {
            self.current_schema = None;
            self.status_msg = self
                .current_catalog
                .as_ref()
                .map(|c| format!("scope: catalog={c}"));
        } else if self.current_catalog.is_some() {
            self.current_catalog = None;
            self.status_msg = None;
        }
        self.refresh_current();
    }

    fn confirm_delete(&mut self) {
        let Some(row) = self.pane().selected() else {
            return;
        };
        let Some(id) = &row.delete_id else {
            self.status_msg = Some(format!("delete not supported on {}", self.tab.title()));
            return;
        };
        let id = id.clone();
        let tab = self.tab;
        self.modal = Modal::Confirm {
            prompt: format!("Delete {} '{id}'?  (y / n)", tab.title()),
            action: PendingAction::Delete {
                tab,
                id,
                force: false,
            },
        };
    }

    fn run_action(&mut self, action: PendingAction) {
        match action {
            PendingAction::Delete { tab, id, force } => self.spawn_delete(tab, id, force),
            PendingAction::Force { tab, id, contents } => {
                self.modal = Modal::Confirm {
                    prompt: sure_prompt(tab, &id, contents),
                    action: PendingAction::Delete {
                        tab,
                        id,
                        force: true,
                    },
                };
            }
        }
    }

    /// `c`: the text input with the prompt of the tab, or a message on the
    /// status line where the tab has no create or needs a scope first.
    fn prompt_create(&mut self) {
        let tab = self.tab;
        match create_prompt(
            tab,
            self.current_catalog.as_deref(),
            self.current_schema.as_deref(),
        ) {
            CreateForm::Prompt(prompt) => {
                self.modal = Modal::TextInput {
                    prompt,
                    value: String::new(),
                    submit: TextSubmit::CreateOnTab(tab),
                };
            }
            CreateForm::Unsupported(msg) => {
                self.status_msg = Some(msg);
            }
            CreateForm::ScopeMissing(msg) => {
                self.status_msg = Some(msg);
            }
        }
    }

    fn run_text_submit(&mut self, submit: TextSubmit, value: String) {
        match submit {
            TextSubmit::SetServer => {
                if value.trim().is_empty() {
                    self.status_msg = Some("server URL must not be empty".to_string());
                    return;
                }
                match config::client(&value, None) {
                    Ok(c) => {
                        // A given token stays with its server; the fetch
                        // sends the sign-in to the new one, if any.
                        self.client = Arc::new(c);
                        self.token = None;
                        self.server_url = value;
                        self.status_msg = Some("server changed — refetching".to_string());
                        // Invalidate every pane so a tab switch refetches.
                        for i in 0..self.panes.len() {
                            self.fetch_gen[i] += 1;
                            self.panes[i].rows.clear();
                        }
                        self.refresh_current();
                    }
                    Err(e) => {
                        self.status_msg = Some(format!("bad URL: {e}"));
                    }
                }
            }
            TextSubmit::CreateOnTab(tab) => {
                if value.trim().is_empty() {
                    self.status_msg = Some("nothing entered — create cancelled".to_string());
                    return;
                }
                if tab == Tab::Tokens {
                    // A second question: when the token expires. The
                    // prompt fits the box of an 80-column screen.
                    self.modal = Modal::TextInput {
                        prompt: "Expiry — days (`30`), a date (`2026-12-31`) or `never`".into(),
                        value: TOKEN_DAYS.to_string(),
                        submit: TextSubmit::TokenExpiry(value.trim().to_string()),
                    };
                    return;
                }
                self.spawn_create(tab, value);
            }
            TextSubmit::TokenExpiry(name) => {
                if value.trim().is_empty() {
                    self.status_msg = Some("no expiry entered — create cancelled".to_string());
                    return;
                }
                self.spawn_create(Tab::Tokens, format!("{name} {}", value.trim()));
            }
        }
    }

    fn spawn_create(&mut self, tab: Tab, raw: String) {
        let client = self.client.clone();
        let tx = self.tx_mut.clone();
        let cat = self.current_catalog.clone();
        let sch = self.current_schema.clone();
        tokio::spawn(async move {
            let outcome = create_row(&client, tab, &raw, cat.as_deref(), sch.as_deref()).await;
            let _ = tx.send(MutationResult { tab, outcome });
        });
    }

    fn refresh_current(&mut self) {
        self.follow_sign_in();
        let tab = self.tab;
        let idx = tab.index();
        self.fetch_gen[idx] += 1;
        let generation = self.fetch_gen[idx];
        self.panes[idx].loading = true;
        self.panes[idx].error = None;

        let client = self.client.clone();
        let tx = self.tx_fetch.clone();
        let cat = self.current_catalog.clone();
        let sch = self.current_schema.clone();
        tokio::spawn(async move {
            let rows = fetch_rows(&client, tab, cat.as_deref(), sch.as_deref()).await;
            let _ = tx.send(FetchResult {
                tab,
                generation,
                rows,
            });
        });
    }

    /// Whether the client sends the sign-in that `uc auth login` keeps for
    /// the server: not with a given token.
    fn follows_sign_in(&self) -> bool {
        self.sign_ins.is_some()
            && !matches!(
                self.token,
                Some((_, TokenSource::Option | TokenSource::Setting))
            )
    }

    /// Send the token of the sign-in to the server, when it changed: a
    /// sign-in in another terminal counts from the next fetch.
    fn follow_sign_in(&mut self) {
        if !self.follows_sign_in() {
            return;
        }
        let (Some(sign_ins), Ok(origin)) = (&self.sign_ins, config::origin(&self.server_url))
        else {
            return;
        };
        let token = sign_ins.token(&origin);
        if token.as_deref() == self.token.as_ref().map(|(token, _)| token.as_str()) {
            return;
        }
        let Ok(client) = config::client(&self.server_url, token.as_deref()) else {
            return;
        };
        self.client = Arc::new(client);
        self.token = token.map(|token| (token, TokenSource::SignIn));
    }

    /// After a 401: how to sign in to the server.
    fn sign_in_advice(&self) -> String {
        let origin = config::origin(&self.server_url).unwrap_or_else(|_| self.server_url.clone());
        let source = self.token.as_ref().map(|(_, source)| *source);
        sign_in::advice(&origin, source, &self.login)
    }

    fn spawn_delete(&mut self, tab: Tab, id: String, force: bool) {
        self.status_msg = Some(format!("deleting {} '{id}'…", tab.title()));
        let client = self.client.clone();
        let tx = self.tx_mut.clone();
        tokio::spawn(async move {
            let outcome = delete_row(&client, tab, &id, force).await;
            let _ = tx.send(MutationResult { tab, outcome });
        });
    }

    fn apply_fetch_result(&mut self, fr: FetchResult) {
        let idx = fr.tab.index();
        if fr.generation != self.fetch_gen[idx] {
            return;
        }
        match fr.rows {
            Ok(rows) => {
                let n = rows.len();
                self.panes[idx].set_rows(rows);
                if self.tab == fr.tab {
                    self.status_msg = Some(format!("loaded {n} {}", fr.tab.title()));
                }
            }
            Err(failure) if failure.needs_sign_in => {
                let mut text = format!("{}\n\n{}", failure.text, self.sign_in_advice());
                if self.follows_sign_in() {
                    text.push_str("\nThen press r.");
                }
                self.panes[idx].set_error(text);
            }
            Err(failure) => self.panes[idx].set_error(failure.text),
        }
    }

    fn apply_mutation_result(&mut self, mr: MutationResult) {
        match mr.outcome {
            Ok(MutationOutcome::Message(msg)) => {
                self.status_msg = Some(msg);
                if self.tab == mr.tab {
                    self.refresh_current();
                }
            }
            Ok(MutationOutcome::Secret {
                title,
                secret,
                hint,
            }) => {
                // The secret shows over the list, which loads again with
                // the new row.
                self.modal = Modal::Secret {
                    title,
                    secret,
                    hint,
                };
                if self.tab == mr.tab {
                    self.refresh_current();
                }
            }
            Ok(MutationOutcome::NotEmpty {
                id,
                reason,
                contents,
            }) => {
                let refused = format!("{} '{id}' not deleted: {reason}", mr.tab.title());
                self.status_msg = Some(refused.clone());
                self.failure = Some(refused);
                self.modal = Modal::Confirm {
                    prompt: not_empty_prompt(&id, &reason, &contents),
                    action: PendingAction::Force {
                        tab: mr.tab,
                        id,
                        contents: contents.ok(),
                    },
                };
            }
            Err(failure) => {
                // The server's message says what failed.
                let text = match failure.needs_sign_in {
                    true => format!("{} {}", failure.text, self.sign_in_advice()),
                    false => failure.text,
                };
                self.status_msg = Some(text.clone());
                self.failure = Some(text);
            }
        }
    }

    fn restore_persisted_state(&mut self) {
        let Some(path) = &self.state_file else { return };
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let Ok(saved) = serde_json::from_slice::<PersistedState>(&bytes) else {
            return;
        };
        if let Some(t) = saved.tab {
            self.tab = t;
        }
        self.current_catalog = saved.current_catalog;
        self.current_schema = saved.current_schema;
        // The server is not restored: it comes from `--server` or the
        // settings.
    }

    fn persist_state(&self) {
        let Some(path) = &self.state_file else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let saved = PersistedState {
            tab: Some(self.tab),
            current_catalog: self.current_catalog.clone(),
            current_schema: self.current_schema.clone(),
        };
        if let Ok(bytes) = serde_json::to_vec_pretty(&saved) {
            let _ = std::fs::write(path, bytes);
        }
    }

    /// Draws the screen in the web UI's colors ([`colors`]), on its white
    /// page, whatever the terminal's own colors are.
    fn render(&mut self, f: &mut Frame<'_>) {
        let area = f.area();
        f.buffer_mut().set_style(area, page_style());
        self.render_widgets(f);
        if !self.true_color {
            for cell in &mut f.buffer_mut().content {
                cell.fg = xterm_256(cell.fg);
                cell.bg = xterm_256(cell.bg);
            }
        }
    }

    fn render_widgets(&mut self, f: &mut Frame<'_>) {
        let area = f.area();

        // Below the minimum size, the bars would take all the rows: the
        // screen only says that the window is too small.
        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            let msg = format!(
                "uc TUI needs at least {MIN_WIDTH}×{MIN_HEIGHT} — current window is {}×{}.\n\
                 Resize the terminal and the UI will reappear.",
                area.width, area.height,
            );
            let p = Paragraph::new(msg).wrap(Wrap { trim: false });
            f.render_widget(p, area);
            return;
        }

        let compact = area.width < COMPACT_WIDTH;
        let stacked = area.width < STACKED_WIDTH;

        // The tab bar and the status line wrap, and take the rows of their
        // text, up to a third of the screen each. The panes get the rest.
        let tabs = self.tab_bar(compact, area.width);
        let status = self.status_bar(compact);
        let rows = |p: &Paragraph<'_>| u16::try_from(p.line_count(area.width)).unwrap_or(u16::MAX);
        let tabs_height = rows(&tabs).min(area.height / 3).max(3);
        let status_height = rows(&status).clamp(1, area.height / 3);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(tabs_height),
                Constraint::Min(1),
                Constraint::Length(status_height),
            ])
            .split(area);

        f.render_widget(tabs, chunks[0]);
        self.render_main(f, chunks[1], stacked);
        f.render_widget(status, chunks[2]);

        if !matches!(self.modal, Modal::None) {
            dim(f.buffer_mut());
        }
        match &self.modal {
            Modal::Help => self.render_help_overlay(f),
            Modal::Confirm { prompt, action } => self.render_confirm_overlay(f, prompt, action),
            Modal::TextInput { prompt, value, .. } => self.render_text_overlay(f, prompt, value),
            Modal::Secret {
                title,
                secret,
                hint,
            } => self.render_secret_overlay(f, title, secret, hint),
            Modal::None => {}
        }
    }

    /// The tab bar of a `width`-column screen: its titles wrap onto more
    /// lines when they do not fit on one ([`tab_lines`]).
    fn tab_bar(&self, compact: bool, width: u16) -> Paragraph<'static> {
        let titles: Vec<&'static str> = Tab::ALL
            .iter()
            .map(|t| if compact { t.short_title() } else { t.title() })
            .collect();
        // The web UI's dark header. It has no line: its border has its color.
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::HEADER));
        let block = if compact {
            block
        } else {
            // The crab stands for Ferris of the web UI's logo.
            block
                .title(" 🦀 uc · Unity Catalog ")
                .title_style(Style::default().fg(colors::WHITE))
        };
        let lines = tab_lines(&titles, self.tab.index(), width.saturating_sub(2).into());
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().fg(colors::HEADER_TEXT).bg(colors::HEADER))
    }

    fn render_main(&mut self, f: &mut Frame<'_>, area: Rect, stacked: bool) {
        let panes = if stacked {
            // A narrow window has the list above the detail.
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(area)
        };

        self.render_list(f, panes[0]);
        self.render_detail(f, panes[1]);
    }

    fn render_list(&mut self, f: &mut Frame<'_>, area: Rect) {
        let pane = &mut self.panes[self.tab.index()];
        let scope_hint = current_scope_hint(
            self.tab,
            self.current_catalog.as_deref(),
            self.current_schema.as_deref(),
        );
        let title = match &scope_hint {
            Some(s) => format!(" {} · {s} ", self.tab.title()),
            None => format!(" {} ", self.tab.title()),
        };
        let block = framed(title);

        if pane.loading && pane.rows.is_empty() {
            let p = Paragraph::new("loading…")
                .block(block)
                .style(Style::default().fg(colors::SECONDARY));
            f.render_widget(p, area);
            return;
        }
        if let Some(err) = &pane.error {
            // The web UI's box for a page that failed to load.
            let p = Paragraph::new(err.clone())
                .block(block.border_style(Style::default().fg(colors::ERROR_BORDER)))
                .style(Style::default().bg(colors::ERROR_BG))
                .wrap(Wrap { trim: false });
            f.render_widget(p, area);
            return;
        }
        if pane.rows.is_empty() {
            let p = Paragraph::new("(empty)")
                .block(block)
                .style(Style::default().fg(colors::SECONDARY));
            f.render_widget(p, area);
            return;
        }

        let items: Vec<ListItem<'_>> = pane
            .rows
            .iter()
            .map(|r| ListItem::new(r.label.as_str()))
            .collect();
        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(colors::SELECTED)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        f.render_stateful_widget(list, area, &mut pane.list_state);
    }

    /// The selected row as JSON. The scroll stops when the last line is at
    /// the bottom. When the JSON is longer than the pane, a scroll bar on
    /// its right line shows which part is in view.
    fn render_detail(&mut self, f: &mut Frame<'_>, area: Rect) {
        let pane = self.pane_mut();
        let block = framed(" Detail ");
        let body = match pane.selected() {
            Some(row) => serde_json::to_string_pretty(&row.detail)
                .unwrap_or_else(|_| "(unserialisable)".to_string()),
            None => String::new(),
        };
        let p = Paragraph::new(body).wrap(Wrap { trim: false });
        let inner = block.inner(area);
        let lines = u16::try_from(p.line_count(inner.width)).unwrap_or(u16::MAX);
        pane.detail_scroll = pane.detail_scroll.min(lines.saturating_sub(inner.height));
        f.render_widget(p.block(block).scroll((pane.detail_scroll, 0)), area);
        if pane.selected().is_none() {
            logo::render(inner, f.buffer_mut());
        }
        scroll_bar(f.buffer_mut(), inner, lines, pane.detail_scroll);
    }

    /// The status line: the server, the last message and the keys, which
    /// wrap onto more lines when they do not fit on one.
    fn status_bar(&self, compact: bool) -> Paragraph<'static> {
        let mut spans = vec![
            Span::styled(
                format!(" {} ", self.server_url),
                Style::default().bg(colors::HEADER).fg(colors::WHITE),
            ),
            Span::raw(" "),
        ];
        if let Some(msg) = &self.status_msg {
            let color = match self.failure.as_ref() == Some(msg) {
                true => colors::DANGER,
                false => colors::SUCCESS,
            };
            spans.push(Span::styled(msg.clone(), Style::default().fg(color)));
            spans.push(Span::raw("  ·  "));
        }
        // In compact mode, only the main keys; `?` shows them all.
        let hint = if compact {
            "c create · d delete · ? help · q quit"
        } else {
            "tab/⇆ switch · ↑↓ nav · ⇧↑↓ scroll · ⏎ drill · ⌫ scope · c create · d delete · s server · r refresh · ? help · q quit"
        };
        spans.push(Span::styled(hint, Style::default().fg(colors::SECONDARY)));
        Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false })
    }

    fn render_help_overlay(&self, f: &mut Frame<'_>) {
        let area = centered_rect(60, 65, 40, 21, f.area());
        f.render_widget(Clear, area);
        let lines = vec![
            Line::from(Span::styled(
                "uc TUI — keys",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(" Tab / →     next tab"),
            Line::from(" Shift-Tab / ← prev tab"),
            Line::from(" ↑ / k       previous row"),
            Line::from(" ↓ / j       next row"),
            Line::from(" Shift-↑/↓   scroll detail pane a line"),
            Line::from(format!(
                " PgUp/PgDn   scroll detail pane {DETAIL_SCROLL_STEP} lines"
            )),
            Line::from(" Enter       drill in (catalog → schemas → tables)"),
            Line::from(" Backspace   drill out one level"),
            Line::from(" c           create a row on the current tab"),
            Line::from(" d           delete the selected row (with confirm)"),
            Line::from(" f           also delete all it holds, when it is not empty"),
            Line::from(" s           change server URL"),
            Line::from(" r           refresh current tab"),
            Line::from(" ?           toggle help"),
            Line::from(" q / Esc     quit (Ctrl-C also works)"),
            Line::from(""),
            Line::from(Span::styled(
                "any key closes this overlay",
                Style::default().fg(colors::SECONDARY),
            )),
        ];
        let p = Paragraph::new(lines)
            .style(page_style())
            .block(framed(" Help "));
        f.render_widget(p, area);
    }

    /// The question of `action`: `prompt`, wrapped, and the keys that
    /// answer it. The box is as high as its text. As the web UI's delete
    /// dialogs, its title and the key that deletes are red.
    fn render_confirm_overlay(&self, f: &mut Frame<'_>, prompt: &str, action: &PendingAction) {
        let (title, yes) = action.question();
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let mut lines = vec![Line::from("")];
        lines.extend(
            prompt
                .lines()
                .map(|line| Line::from(Span::styled(line.to_string(), bold))),
        );
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(yes, Style::default().fg(colors::DANGER)),
            Span::raw("    "),
            Span::styled("n / Esc = cancel", Style::default().fg(colors::SECONDARY)),
        ]));
        let p = Paragraph::new(lines)
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: false })
            .style(page_style())
            .block(framed(title).title_style(Style::default().fg(colors::DANGER)));
        let width = centered_rect(50, 0, 36, 0, f.area()).width;
        let height = u16::try_from(p.line_count(width.saturating_sub(2))).unwrap_or(u16::MAX);
        let area = centered_rect(0, 0, width, height.max(6), f.area());
        f.render_widget(Clear, area);
        f.render_widget(p, area);
    }

    fn render_secret_overlay(&self, f: &mut Frame<'_>, title: &str, secret: &str, hint: &str) {
        let area = centered_rect(70, 30, 50, 9, f.area());
        f.render_widget(Clear, area);
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                title.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            // As code in the web UI, on grey.
            Line::from(Span::styled(
                secret.to_string(),
                Style::default()
                    .bg(colors::CODE_BG)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            // As the web UI's warnings, on yellow.
            Line::from(Span::styled(
                hint.to_string(),
                Style::default().bg(colors::WARNING_BG),
            )),
        ];
        let p = Paragraph::new(lines)
            .style(page_style())
            .block(framed(" Secret "))
            .wrap(Wrap { trim: false });
        f.render_widget(p, area);
    }

    fn render_text_overlay(&self, f: &mut Frame<'_>, prompt: &str, value: &str) {
        let area = centered_rect(70, 25, 40, 8, f.area());
        f.render_widget(Clear, area);
        let cursor = match self.cursor_shown {
            true => "_",
            false => " ",
        };
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                prompt.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            // Blue, as the web UI's input with the focus and its OK button.
            Line::from(Span::styled(
                format!("▸ {value}{cursor}"),
                Style::default().fg(colors::PRIMARY),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Enter = submit", Style::default().fg(colors::PRIMARY)),
                Span::raw("    "),
                Span::styled("Esc = cancel", Style::default().fg(colors::SECONDARY)),
            ]),
        ];
        let p = Paragraph::new(lines)
            .style(page_style())
            .block(framed(" Input "));
        f.render_widget(p, area);
    }
}

#[derive(Serialize, Deserialize, Default)]
struct PersistedState {
    #[serde(default)]
    tab: Option<Tab>,
    #[serde(default)]
    current_catalog: Option<String>,
    #[serde(default)]
    current_schema: Option<String>,
}

fn state_path() -> Option<PathBuf> {
    config::state_dir().map(|dir| dir.join("state.json"))
}

/// The tab bar's lines, at most `width` columns each: every title between
/// spaces, `│` between titles, and a new line where the next title does not
/// fit, so that no title is cut. The `selected` title and its spaces are
/// white on blue, as the web UI's selected menu item.
fn tab_lines(titles: &[&'static str], selected: usize, width: usize) -> Vec<Line<'static>> {
    let highlight = Style::default().fg(colors::WHITE).bg(colors::PRIMARY);
    let mut lines = Vec::new();
    let mut line: Vec<Span<'static>> = Vec::new();
    let mut used = 0;
    for (i, title) in titles.iter().enumerate() {
        let style = match i == selected {
            true => highlight,
            false => Style::default(),
        };
        let title = Span::styled(*title, style);
        let cell = title.width() + 2;
        if used > 0 && used + 1 + cell > width {
            lines.push(Line::from(std::mem::take(&mut line)));
            used = 0;
        }
        if used > 0 {
            line.push(Span::raw(symbols::line::VERTICAL));
            used += 1;
        }
        line.extend([Span::styled(" ", style), title, Span::styled(" ", style)]);
        used += cell;
    }
    lines.push(Line::from(line));
    lines
}

fn current_scope_hint(tab: Tab, cat: Option<&str>, sch: Option<&str>) -> Option<Cow<'static, str>> {
    if tab.needs_schema_scope() {
        match (cat, sch) {
            (Some(c), Some(s)) => Some(Cow::Owned(format!("{c}.{s}"))),
            _ => Some(Cow::Borrowed(
                "⚠ pick a catalog+schema first (Enter on rows)",
            )),
        }
    } else if tab.needs_catalog_scope() {
        match cat {
            Some(c) => Some(Cow::Owned(c.to_string())),
            None => Some(Cow::Borrowed("⚠ pick a catalog first (Enter on a row)")),
        }
    } else {
        None
    }
}

/// A rect in the middle of `r`: a percentage of its size, but at least
/// `min_w` × `min_h` within `r`, so that a modal stays readable on a small
/// terminal.
fn centered_rect(percent_x: u16, percent_y: u16, min_w: u16, min_h: u16, r: Rect) -> Rect {
    let w = ((r.width as u32 * percent_x as u32 / 100) as u16)
        .max(min_w)
        .min(r.width);
    let h = ((r.height as u32 * percent_y as u32 / 100) as u16)
        .max(min_h)
        .min(r.height);
    let x = r.x + r.width.saturating_sub(w) / 2;
    let y = r.y + r.height.saturating_sub(h) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}

/// The web UI's colors (`crates/uc-ui/public/styles.css`): antd 5's
/// default theme. A color with transparency is given as it shows on its
/// background.
mod colors {
    use ratatui::style::Color;

    /// The page and the dialogs.
    pub const PAGE: Color = Color::Rgb(0xff, 0xff, 0xff);
    /// Black at 88% on the page.
    pub const TEXT: Color = Color::Rgb(0x1f, 0x1f, 0x1f);
    /// Black at 45% on the page: hints, and lists that load or are empty.
    pub const SECONDARY: Color = Color::Rgb(0x8c, 0x8c, 0x8c);
    pub const BORDER: Color = Color::Rgb(0xd9, 0xd9, 0xd9);
    /// The selected item of the sidebar.
    pub const SELECTED: Color = Color::Rgb(0xe6, 0xf4, 0xff);
    pub const HEADER: Color = Color::Rgb(0x00, 0x15, 0x29);
    /// White at 65% on the header: its menu items.
    pub const HEADER_TEXT: Color = Color::Rgb(0xa6, 0xad, 0xb4);
    pub const WHITE: Color = Color::Rgb(0xff, 0xff, 0xff);
    /// The selected menu item, the OK buttons and the input with the focus.
    pub const PRIMARY: Color = Color::Rgb(0x16, 0x77, 0xff);
    /// Delete dialogs and failures.
    pub const DANGER: Color = Color::Rgb(0xff, 0x4d, 0x4f);
    pub const SUCCESS: Color = Color::Rgb(0x52, 0xc4, 0x1a);
    /// A page that failed to load: antd's error `Alert`.
    pub const ERROR_BG: Color = Color::Rgb(0xff, 0xf2, 0xf0);
    pub const ERROR_BORDER: Color = Color::Rgb(0xff, 0xcc, 0xc7);
    /// antd's warning `Alert`.
    pub const WARNING_BG: Color = Color::Rgb(0xff, 0xfb, 0xe6);
    /// Code: grey at 10% on the page.
    pub const CODE_BG: Color = Color::Rgb(0xf5, 0xf5, 0xf5);
}

/// A scroll bar on the line right of `inner`, for a text of `lines` lines
/// in view from line `top`: a thick grey thumb on the light line, none
/// when all of the text is in view. Unlike ratatui's `Scrollbar`, which
/// rounds both ends of the thumb, the thumb keeps one size as it moves.
fn scroll_bar(buffer: &mut Buffer, inner: Rect, lines: u16, top: u16) {
    let (view, lines) = (u32::from(inner.height), u32::from(lines));
    if view == 0 || lines <= view {
        return;
    }
    // The thumb is to the bar as the view is to the text, and it is at the
    // bottom when the last line is.
    let thumb = ((view * view + lines / 2) / lines).clamp(1, view);
    let last = lines - view;
    let start = (u32::from(top).min(last) * (view - thumb) + last / 2) / last;
    for i in 0..inner.height {
        let (symbol, color) = match (start..start + thumb).contains(&u32::from(i)) {
            true => (symbols::line::THICK_VERTICAL, colors::SECONDARY),
            false => (symbols::line::VERTICAL, colors::BORDER),
        };
        buffer[(inner.right(), inner.y + i)]
            .set_symbol(symbol)
            .set_fg(color);
    }
}

/// Dark text on white, as the web UI's page.
fn page_style() -> Style {
    Style::default().fg(colors::TEXT).bg(colors::PAGE)
}

/// A box with `title` and a light line, as the web UI's panels.
fn framed<'a>(title: impl Into<Line<'a>>) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::BORDER))
        .title(title.into())
        .title_style(Style::default().fg(colors::TEXT))
}

/// Darkens the screen behind a dialog as the web UI does: black at 45%
/// over it.
fn dim(buffer: &mut Buffer) {
    let darker = |c: u8| (u16::from(c) * 55 / 100) as u8;
    let dim = |color| match color {
        Color::Rgb(r, g, b) => Color::Rgb(darker(r), darker(g), darker(b)),
        color => color,
    };
    for cell in &mut buffer.content {
        cell.fg = dim(cell.fg);
        cell.bg = dim(cell.bg);
    }
}

/// The nearest of xterm's 256 colors to `color`, for terminals without
/// 24-bit colors: one of its 6×6×6 cube, or of its 24 greys.
fn xterm_256(color: Color) -> Color {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    let Color::Rgb(r, g, b) = color else {
        return color;
    };
    let distance = |[x, y, z]: [u8; 3]| {
        [r.abs_diff(x), g.abs_diff(y), b.abs_diff(z)]
            .map(|d| u32::from(d).pow(2))
            .iter()
            .sum::<u32>()
    };
    // The cube's nearest level of red, of green and of blue.
    let level = |c: u8| {
        (0..6u8)
            .min_by_key(|&i| LEVELS[usize::from(i)].abs_diff(c))
            .unwrap_or(0)
    };
    let [ri, gi, bi] = [r, g, b].map(level);
    let cube = [ri, gi, bi].map(|i| LEVELS[usize::from(i)]);
    // The nearest grey: grey i is 8 + 10 i.
    let average = (u16::from(r) + u16::from(g) + u16::from(b)) / 3;
    let grey = (average.saturating_sub(3) / 10).min(23) as u8;
    match distance([8 + 10 * grey; 3]) < distance(cube) {
        true => Color::Indexed(232 + grey),
        false => Color::Indexed(16 + 36 * ri + 6 * gi + bi),
    }
}

async fn fetch_rows(
    client: &Client,
    tab: Tab,
    cat: Option<&str>,
    sch: Option<&str>,
) -> Result<Vec<Row>, Failure> {
    match tab {
        Tab::Catalogs => {
            let resp = client.list_catalogs().await?;
            Ok(resp
                .catalogs
                .into_iter()
                .map(|c| Row {
                    label: c.name.clone(),
                    delete_id: named(&c.name),
                    detail: detail(&models::CATALOG, &c),
                })
                .collect())
        }
        Tab::Schemas => {
            let cat = cat.ok_or("select a catalog first (Catalogs tab + Enter)")?;
            let resp = client.list_schemas(cat).await?;
            Ok(resp
                .schemas
                .into_iter()
                .map(|s| Row {
                    label: label(&s.full_name, &s.name),
                    delete_id: named(&s.full_name),
                    detail: detail(&models::SCHEMA, &s),
                })
                .collect())
        }
        Tab::Tables => {
            let (c, s) = scope_or_err(cat, sch)?;
            let resp = client.list_tables(c, s).await?;
            Ok(resp
                .tables
                .into_iter()
                .map(|t| {
                    let full = three_part_name(
                        nonempty(&t.catalog_name),
                        nonempty(&t.schema_name),
                        nonempty(&t.name),
                    );
                    let delete_id = (full != "(unnamed)").then(|| full.clone());
                    Row {
                        label: full,
                        delete_id,
                        detail: detail(&models::TABLE, &t),
                    }
                })
                .collect())
        }
        Tab::Volumes => {
            let (c, s) = scope_or_err(cat, sch)?;
            let resp = client.list_volumes(c, s).await?;
            Ok(resp
                .volumes
                .into_iter()
                .map(|v| Row {
                    label: label(&v.full_name, &v.name),
                    delete_id: named(&v.full_name),
                    detail: to_value(&v),
                })
                .collect())
        }
        Tab::Functions => {
            let (c, s) = scope_or_err(cat, sch)?;
            let resp = client.list_functions(c, s).await?;
            Ok(resp
                .functions
                .into_iter()
                .map(|fnc| Row {
                    label: label(&fnc.full_name, &fnc.name),
                    delete_id: named(&fnc.full_name),
                    detail: to_value(&fnc),
                })
                .collect())
        }
        Tab::Models => {
            let resp = client.list_registered_models().await?;
            Ok(resp
                .registered_models
                .into_iter()
                .map(|m| Row {
                    label: label(&m.full_name, &m.name),
                    delete_id: named(&m.full_name),
                    detail: to_value(&m),
                })
                .collect())
        }
        Tab::Credentials => {
            let resp = client.list_credentials().await?;
            Ok(resp
                .credentials
                .into_iter()
                .map(|c| Row {
                    label: nonempty(&c.name).unwrap_or("(unnamed)").to_string(),
                    delete_id: named(&c.name),
                    detail: to_value(&c),
                })
                .collect())
        }
        Tab::ExternalLocations => {
            let resp = client.list_external_locations().await?;
            Ok(resp
                .external_locations
                .into_iter()
                .map(|el| Row {
                    label: format!("{}  →  {}", el.name, el.url),
                    delete_id: named(&el.name),
                    detail: to_value(&el),
                })
                .collect())
        }
        Tab::Namespaces => {
            // Iceberg REST namespaces of the current catalog.
            let cat = cat.ok_or("select a catalog first (Catalogs tab + Enter)")?;
            let resp = client.list_iceberg_namespaces(cat).await?;
            Ok(resp
                .namespaces
                .into_iter()
                .map(|ns| {
                    let levels: Vec<&str> = ns.values.iter().filter_map(|v| v.as_str()).collect();
                    Row {
                        label: format!("{cat}.{}", levels.join(".")),
                        delete_id: None,
                        detail: to_value(&ns),
                    }
                })
                .collect())
        }
        Tab::ScimUsers => {
            let resp = client.list_users().await?;
            Ok(resp
                .resources
                .into_iter()
                .map(|u| Row {
                    label: format!("{}  ({})", u.user_name, u.id),
                    delete_id: named(&u.id),
                    detail: to_value(&u),
                })
                .collect())
        }
        Tab::Tokens => {
            let resp = client.list_tokens().await?;
            Ok(resp
                .tokens
                .into_iter()
                .map(|p| Row {
                    label: format!("{}  {}  ({})", p.name, expiry_label(p.expires_at), p.id),
                    delete_id: named(&p.id),
                    detail: to_value(&p),
                })
                .collect())
        }
    }
}

/// Delete `id` on `tab`; with `force`, a catalog, schema or model goes
/// with all it holds. Without `force`, the server keeps one that holds
/// something: the outcome is then [`MutationOutcome::NotEmpty`], with what
/// it holds.
async fn delete_row(
    client: &Client,
    tab: Tab,
    id: &str,
    force: bool,
) -> Result<MutationOutcome, Failure> {
    let kind = tab.title();
    let res = match tab {
        Tab::Catalogs => client.catalog(id).delete().with_force(force).await,
        Tab::Schemas => client.schema(id).delete().with_force(force).await,
        Tab::Tables => client.table(id).delete().await,
        Tab::Volumes => client.volume(id).delete().await,
        Tab::Functions => client.function(id).delete().await,
        Tab::Models => client.registered_model(id).delete().with_force(force).await,
        Tab::Credentials => client.credential(id).delete().with_force(false).await,
        Tab::ExternalLocations => {
            client
                .external_location(id)
                .delete()
                .with_force(false)
                .await
        }
        Tab::Tokens => client.token(id).delete().await,
        Tab::ScimUsers => client.delete_user(id).await,
        Tab::Namespaces => return Err("drop_namespace not supported from TUI".into()),
    };
    match res {
        Ok(()) if force => Ok(MutationOutcome::Message(format!(
            "deleted {kind} '{id}' and all it held"
        ))),
        Ok(()) => Ok(MutationOutcome::Message(format!("deleted {kind} '{id}'"))),
        // The server's answer for a catalog or schema that holds something
        // (FAILED_PRECONDITION), and for a model with versions (ABORTED).
        Err(e)
            if !force
                && tab.holder().is_some()
                && matches!(
                    call_error(&e).code(),
                    ErrorCode::FailedPrecondition | ErrorCode::Aborted
                ) =>
        {
            Ok(MutationOutcome::NotEmpty {
                id: id.to_string(),
                reason: message(&e),
                contents: contents(client, tab, id).await.map_err(|e| message(&e)),
            })
        }
        Err(e) => Err(e.into()),
    }
}

/// What the catalog, schema or model `id` on `tab` holds.
async fn contents(client: &Client, tab: Tab, id: &str) -> uc_client::error::Result<Contents> {
    match tab {
        Tab::Catalogs => client.catalog_contents(id).await,
        Tab::Schemas => {
            let (catalog, schema) = id.split_once('.').unwrap_or((id, ""));
            client.schema_contents(catalog, schema).await
        }
        _ => client.model_contents(id).await,
    }
}

/// The question after the server refused to delete `id`: why, and what it
/// holds.
fn not_empty_prompt(id: &str, reason: &str, contents: &Result<Contents, String>) -> String {
    let held = match contents {
        Ok(contents) => match contents.describe() {
            Some(text) => format!("{id} holds {text}."),
            None => format!("{id} holds nothing else now."),
        },
        Err(e) => format!("uc could not count what {id} holds: {e}"),
    };
    format!("{reason}\n\n{held}")
}

/// The last question before the delete of `id` on `tab` with all it holds,
/// as the web UI asks it.
fn sure_prompt(tab: Tab, id: &str, contents: Option<Contents>) -> String {
    let what = tab.holder().unwrap_or("object");
    let held = match contents.and_then(|contents| contents.describe()) {
        Some(text) => format!(" and all it holds: {text}"),
        None => " and all it holds".to_string(),
    };
    let data = match tab {
        Tab::Models => "The data of its versions goes too.",
        _ => "The data of managed tables and volumes, and of models, goes too.",
    };
    format!("This deletes the {what} {id}{held}. {data}\n\nThis operation cannot be undone.")
}

fn scope_or_err<'a>(
    cat: Option<&'a str>,
    sch: Option<&'a str>,
) -> Result<(&'a str, &'a str), String> {
    match (cat, sch) {
        (Some(c), Some(s)) => Ok((c, s)),
        _ => Err("select a schema first (Catalogs → Enter, then Schemas → Enter)".to_string()),
    }
}

fn three_part_name(cat: Option<&str>, sch: Option<&str>, name: Option<&str>) -> String {
    match (cat, sch, name) {
        (Some(c), Some(s), Some(n)) => format!("{c}.{s}.{n}"),
        (_, _, Some(n)) => n.to_string(),
        _ => "(unnamed)".to_string(),
    }
}

/// The text of a name; none when the server left it out.
fn nonempty(text: &str) -> Option<&str> {
    Some(text).filter(|text| !text.is_empty())
}

/// [`nonempty`], owned.
fn named(text: &str) -> Option<String> {
    nonempty(text).map(str::to_string)
}

/// The label of a row: its full name, else its name.
fn label(full_name: &str, name: &str) -> String {
    nonempty(full_name)
        .or(nonempty(name))
        .unwrap_or_default()
        .to_string()
}

fn to_value<T: serde::Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

/// An object of `model`'s type as the detail pane shows it: as the server
/// sends it, with its empty maps and lists ([`with_empty`]).
fn detail<T: serde::Serialize>(model: &Model, v: &T) -> serde_json::Value {
    with_empty(model, to_value(v))
}

// A create takes one line of input: a name, or a few fields between
// spaces. Tables and functions need columns and parameters, so for them
// the TUI names the `uc` command instead.

enum CreateForm {
    /// Show the text-input modal with this prompt. Submit triggers `create_row`.
    Prompt(String),
    /// The tab has no create: Iceberg namespaces are made on the Schemas
    /// tab, and tables and functions need more than one line.
    Unsupported(String),
    /// The tab needs a scope first, such as a catalog for Schemas.
    ScopeMissing(String),
}

fn create_prompt(tab: Tab, cat: Option<&str>, sch: Option<&str>) -> CreateForm {
    match tab {
        Tab::Catalogs => CreateForm::Prompt("New catalog — enter `<name> [comment...]`".into()),
        Tab::Schemas => match cat {
            Some(c) => CreateForm::Prompt(format!(
                "New schema under `{c}` — enter `<name> [comment...]`"
            )),
            None => {
                CreateForm::ScopeMissing("select a catalog first (Catalogs tab → Enter)".into())
            }
        },
        Tab::Volumes => match (cat, sch) {
            (Some(c), Some(s)) => CreateForm::Prompt(format!(
                "New volume under `{c}.{s}` — enter `<name> [MANAGED|EXTERNAL] [storage_url]`"
            )),
            _ => CreateForm::ScopeMissing(
                "select a schema first (Catalogs → Enter → Schemas → Enter)".into(),
            ),
        },
        Tab::Models => match (cat, sch) {
            (Some(c), Some(s)) => CreateForm::Prompt(format!(
                "New model under `{c}.{s}` — enter `<name> [comment...]`"
            )),
            _ => CreateForm::ScopeMissing(
                "select a schema first (Catalogs → Enter → Schemas → Enter)".into(),
            ),
        },
        Tab::Credentials => CreateForm::Prompt(
            "New credential — enter `<name> [STORAGE|SERVICE_PRINCIPAL] [aws_role_arn]`".into(),
        ),
        Tab::ExternalLocations => CreateForm::Prompt(
            "New external location — enter `<name> <url> <credential_name>`".into(),
        ),
        Tab::Tokens => CreateForm::Prompt("New PAT — enter `<name>`; its expiry comes next".into()),
        Tab::ScimUsers => {
            CreateForm::Prompt("New SCIM user — enter `<email> [displayName...]`".into())
        }
        Tab::Tables => CreateForm::Unsupported(
            "tables need a column schema — use `uc table` from a shell".into(),
        ),
        Tab::Functions => CreateForm::Unsupported(
            "functions need a parameter signature — use `uc function` from a shell".into(),
        ),
        Tab::Namespaces => CreateForm::Unsupported(
            "Iceberg namespaces are UC schemas — create one in the Schemas tab".into(),
        ),
    }
}

/// The input split on whitespace into at most `n` parts. The last part is
/// the rest with its spaces, so that a comment with spaces is one field.
fn split_create_input(raw: &str, n: usize) -> Vec<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<String> = Vec::with_capacity(n);
    let mut rest = raw;
    for _ in 0..n.saturating_sub(1) {
        match rest.split_once(char::is_whitespace) {
            Some((head, tail)) => {
                out.push(head.to_string());
                rest = tail.trim_start();
                if rest.is_empty() {
                    return out;
                }
            }
            None => break,
        }
    }
    if !rest.is_empty() {
        out.push(rest.to_string());
    }
    out
}

/// The expiry of a new token, as the TUI asks for it: `n` days after
/// `now`, the end of a date (`2026-12-31`) in the local time zone, or
/// `never`. In epoch milliseconds; none for `never`.
fn token_expiry(text: &str, now: DateTime<Local>) -> Result<Option<i64>, String> {
    const DAY_MS: i64 = 24 * 60 * 60 * 1000;
    let text = text.trim();
    if text.eq_ignore_ascii_case("never") {
        return Ok(None);
    }
    if let Ok(days) = text.parse::<u32>() {
        if days == 0 {
            return Err("the expiry must be at least 1 day".into());
        }
        return Ok(Some(now.timestamp_millis() + i64::from(days) * DAY_MS));
    }
    let date = NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| {
        format!("expiry must be days (`30`), a date (`2026-12-31`) or `never`, got '{text}'")
    })?;
    let end = date
        .and_hms_milli_opt(23, 59, 59, 999)
        .and_then(|end| Local.from_local_datetime(&end).earliest())
        .ok_or_else(|| format!("{date} has no end in the local time zone"))?;
    if end <= now {
        return Err(format!("the date {date} has passed"));
    }
    Ok(Some(end.timestamp_millis()))
}

/// A token's expiry in its row, in local time.
fn expiry_label(expires_at: Option<i64>) -> String {
    let at = expires_at
        .filter(|&ms| ms != 0)
        .and_then(|ms| Local.timestamp_millis_opt(ms).single());
    let Some(at) = at else {
        return "never expires".into();
    };
    let shown = at.format("%Y-%m-%d %H:%M");
    match at <= Local::now() {
        true => format!("expired {shown}"),
        false => format!("expires {shown}"),
    }
}

async fn create_row(
    client: &Client,
    tab: Tab,
    raw: &str,
    cat: Option<&str>,
    sch: Option<&str>,
) -> Result<MutationOutcome, Failure> {
    fn msg(s: String) -> Result<MutationOutcome, Failure> {
        Ok(MutationOutcome::Message(s))
    }
    fn name_comment(raw: &str) -> Result<(String, Option<String>), String> {
        let parts = split_create_input(raw, 2);
        let name = parts.first().ok_or("name required")?.clone();
        let comment = parts.get(1).cloned();
        Ok((name, comment))
    }

    match tab {
        Tab::Catalogs => {
            let (name, comment) = name_comment(raw)?;
            let info = client.create_catalog(name).with_comment(comment).await?;
            msg(format!("created catalog '{}'", info.name))
        }
        Tab::Schemas => {
            let c = cat.ok_or("no current catalog scope")?;
            let (name, comment) = name_comment(raw)?;
            let info = client.create_schema(name, c).with_comment(comment).await?;
            msg(format!("created schema '{}'", info.full_name))
        }
        Tab::Volumes => {
            let (c, s) = match (cat, sch) {
                (Some(c), Some(s)) => (c, s),
                _ => return Err("select a schema first".into()),
            };
            let parts = split_create_input(raw, 3);
            let name = parts.first().ok_or("name required")?.clone();
            let vt = match parts.get(1).map(String::as_str) {
                None | Some("") | Some("MANAGED") => VolumeType::Managed,
                Some("EXTERNAL") => VolumeType::External,
                Some(other) => {
                    return Err(
                        format!("volume_type must be MANAGED or EXTERNAL, got '{other}'").into(),
                    );
                }
            };
            let storage_location = parts.get(2).cloned();
            let info = client
                .create_volume(c, s, name, vt)
                .with_storage_location(storage_location)
                .await?;
            msg(format!(
                "created volume '{}'",
                three_part_name(
                    nonempty(&info.catalog_name),
                    nonempty(&info.schema_name),
                    nonempty(&info.name)
                )
            ))
        }
        Tab::Models => {
            let (c, s) = match (cat, sch) {
                (Some(c), Some(s)) => (c, s),
                _ => return Err("select a schema first".into()),
            };
            let (name, comment) = name_comment(raw)?;
            let info = client
                .create_registered_model(name, c, s)
                .with_comment(comment)
                .await?;
            msg(format!("created model '{}'", info.full_name))
        }
        Tab::Credentials => {
            let parts = split_create_input(raw, 3);
            let name = parts.first().ok_or("name required")?.clone();
            let purpose = match parts.get(1).map(String::as_str) {
                None | Some("") => None,
                Some("STORAGE") => Some(CredentialPurpose::Storage),
                Some("SERVICE_PRINCIPAL") => Some(CredentialPurpose::ServicePrincipal),
                Some(other) => {
                    return Err(format!(
                        "purpose must be STORAGE or SERVICE_PRINCIPAL, got '{other}'"
                    )
                    .into());
                }
            };
            // The API takes a role: without one in the input, its ARN is
            // empty.
            let role = AwsIamRoleRequest {
                role_arn: parts.get(2).cloned().unwrap_or_default(),
                ..Default::default()
            };
            let info = client
                .create_credential(name, role)
                .with_purpose(purpose)
                .await?;
            msg(format!("created credential '{}'", info.name))
        }
        Tab::ExternalLocations => {
            let parts = split_create_input(raw, 3);
            if parts.len() < 3 {
                return Err("expected `<name> <url> <credential_name>`".into());
            }
            let info = client
                .create_external_location(&parts[0], &parts[1], &parts[2])
                .await?;
            msg(format!("created external_location '{}'", info.name))
        }
        Tab::Tokens => {
            // `<name> <expiry>`: the name may hold spaces, and the expiry
            // is the last word ([`token_expiry`]).
            let (name, expiry) = raw
                .trim()
                .rsplit_once(char::is_whitespace)
                .ok_or("expected `<name> <expiry>`")?;
            let name = name.trim().to_string();
            let expires_at = token_expiry(expiry, Local::now())?;
            let info = client
                .create_token(name)
                .with_expires_at(expires_at)
                .await?;
            // This is the only time the value shows: the list of tokens
            // does not return it. A modal shows it, not the status line,
            // which stays on the screen.
            let secret = info
                .token_value
                .clone()
                .unwrap_or_else(|| "(no token returned)".into());
            Ok(MutationOutcome::Secret {
                title: format!("PAT '{}' created", info.name),
                secret,
                hint: "This token is shown ONCE. Copy it now — press Enter or Esc to dismiss."
                    .into(),
            })
        }
        Tab::ScimUsers => {
            let parts = split_create_input(raw, 2);
            let user_name = parts.first().ok_or("userName required")?.clone();
            let display_name = parts.get(1).cloned();
            // The server names the user by its primary email.
            let email = ScimEmail {
                value: user_name.clone(),
                primary: Some(true),
                ..Default::default()
            };
            let resp = client
                .create_user()
                .with_display_name(display_name)
                .with_emails([email])
                .await?;
            let id = nonempty(&resp.id).unwrap_or("?");
            msg(format!("created SCIM user '{user_name}' (id {id})"))
        }
        Tab::Tables | Tab::Functions | Tab::Namespaces => {
            // The keys do not get here: these tabs have no prompt
            // ([`create_prompt`]).
            Err("create not supported on this tab".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::OnceLock;
    use std::time::Duration;

    use crossterm::event::KeyEvent;
    use ratatui::backend::TestBackend;
    use serde_json::json;

    use uc_client::error::ApiError;
    use uc_client::models::tables::v1::{DataSourceFormat, TableType};
    use uc_metastore::{MetastoreConfig, SqlMetastore};
    use uc_server::authn::security::{ADMIN, SecurityContext};
    use uc_server::{AppState, build_app};

    use super::*;

    /// The server signing key, made once per test binary.
    fn security() -> Arc<SecurityContext> {
        static SECURITY: OnceLock<Arc<SecurityContext>> = OnceLock::new();
        SECURITY
            .get_or_init(|| Arc::new(SecurityContext::generate().unwrap()))
            .clone()
    }

    /// A uc-server; its origin.
    async fn server() -> String {
        let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
            .await
            .expect("open the test database");
        let app = build_app(AppState::new(metastore).with_security(security()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        format!("http://{addr}")
    }

    /// The API client of `origin`, signed in as the admin.
    fn client(origin: &str) -> Client {
        let token = security()
            .create_access_token(ADMIN, std::time::Duration::from_secs(3600))
            .unwrap();
        config::client(&format!("{origin}{API_BASE}/"), Some(&token)).unwrap()
    }

    /// The table `c.s.t`.
    async fn external_table(api: &Client) {
        api.create_table("t", "c", "s", TableType::External)
            .with_data_source_format(DataSourceFormat::Delta)
            .with_storage_location("s3://lake/t".to_string())
            .await
            .unwrap();
    }

    /// The function `c.s.f`.
    async fn python_function(api: &Client) {
        let function = serde_json::from_value(json!({
            "name": "f", "catalog_name": "c", "schema_name": "s",
            "input_params": { "parameters": [] }, "data_type": "INT",
            "full_data_type": "INT", "routine_body": "EXTERNAL",
            "routine_definition": "return 1", "parameter_style": "S",
            "is_deterministic": true, "sql_data_access": "NO_SQL",
            "is_null_call": true, "security_type": "DEFINER",
            "specific_name": "f", "external_language": "python",
        }))
        .unwrap();
        api.create_function(function).await.unwrap();
    }

    fn app(origin: &str) -> App {
        App::new(client(origin), format!("{origin}{API_BASE}/"), None)
    }

    /// An app whose server does not answer, for what needs none.
    fn offline() -> App {
        app("http://127.0.0.1:9")
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_event(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    /// Apply the next result of a background fetch or mutation.
    async fn settle(app: &mut App) {
        tokio::time::timeout(Duration::from_secs(10), async {
            tokio::select! {
                Some(fr) = app.rx_fetch.recv() => app.apply_fetch_result(fr),
                Some(mr) = app.rx_mut.recv() => app.apply_mutation_result(mr),
            }
        })
        .await
        .expect("a fetch or mutation result");
    }

    /// Switch to `tab` with the arrow keys and load it.
    async fn go_to(app: &mut App, tab: Tab) {
        while app.tab != tab {
            press(app, KeyCode::Right);
            settle(app).await;
        }
    }

    fn labels(app: &App) -> Vec<&str> {
        app.pane().rows.iter().map(|r| r.label.as_str()).collect()
    }

    fn status(app: &App) -> &str {
        app.status_msg.as_deref().unwrap_or_default()
    }

    /// Create a row with the `c` key and the text `input`; the status.
    async fn create(app: &mut App, input: &str) -> String {
        press(app, KeyCode::Char('c'));
        assert!(
            matches!(app.modal, Modal::TextInput { .. }),
            "{}",
            status(app)
        );
        type_text(app, input);
        press(app, KeyCode::Enter);
        settle(app).await;
        let shown = status(app).to_string();
        if shown.starts_with("created") {
            // The tab reloads.
            settle(app).await;
        }
        shown
    }

    /// Delete the selected row with `d`, confirmed with `y`; the status.
    async fn delete(app: &mut App) -> String {
        press(app, KeyCode::Char('d'));
        assert!(
            matches!(app.modal, Modal::Confirm { .. }),
            "{}",
            status(app)
        );
        press(app, KeyCode::Char('y'));
        settle(app).await;
        let shown = status(app).to_string();
        if shown.starts_with("deleted") {
            settle(app).await;
        }
        shown
    }

    /// The catalog `c`, with a storage root for managed models.
    async fn managed_catalog(api: &Client, dir: &std::path::Path) {
        api.create_catalog("c")
            .with_storage_root(format!("file://{}/root", dir.display()))
            .await
            .unwrap();
    }

    /// A folder for an external volume, outside managed storage.
    fn external_folder(dir: &std::path::Path) -> String {
        let folder = dir.join("external");
        std::fs::create_dir_all(&folder).unwrap();
        format!("file://{}", folder.display())
    }

    fn row(label: &str) -> Row {
        Row {
            label: label.to_string(),
            detail: json!({ "name": label }),
            delete_id: Some(label.to_string()),
        }
    }

    /// The cells of the screen of `app` at `width` × `height`.
    fn buffer(app: &mut App, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| app.render(f)).unwrap();
        terminal.backend().buffer().clone()
    }

    /// The screen of `app` at `width` × `height`, one string per line.
    fn screen(app: &mut App, width: u16, height: u16) -> Vec<String> {
        let buffer = buffer(app, width, height);
        (0..height)
            .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
            .collect()
    }

    /// The column and line of the first `text` on the screen in `buffer`.
    fn position(buffer: &Buffer, text: &str) -> (u16, u16) {
        let area = buffer.area;
        for y in 0..area.height {
            let line: String = (0..area.width).map(|x| buffer[(x, y)].symbol()).collect();
            if let Some(i) = line.find(text) {
                return (u16::try_from(line[..i].chars().count()).unwrap(), y);
            }
        }
        panic!("{text:?} is not on the screen");
    }

    fn shows(screen: &[String], text: &str) -> bool {
        screen.iter().any(|line| line.contains(text))
    }

    #[test]
    fn tabs_go_round() {
        assert_eq!(Tab::Catalogs.prev(), Tab::Tokens);
        assert_eq!(Tab::Tokens.next(), Tab::Catalogs);
        for (i, tab) in Tab::ALL.iter().enumerate() {
            assert_eq!(tab.index(), i);
            assert_eq!(tab.next().prev(), *tab);
            assert!(tab.short_title().len() <= 3);
        }
        let titles: std::collections::HashSet<_> = Tab::ALL.iter().map(|t| t.title()).collect();
        assert_eq!(titles.len(), Tab::ALL.len());
    }

    #[test]
    fn create_input_splits_on_whitespace() {
        assert_eq!(split_create_input("  a  b c d ", 2), ["a", "b c d"]);
        assert_eq!(split_create_input("a b", 1), ["a b"]);
        assert_eq!(split_create_input("a", 3), ["a"]);
        assert_eq!(split_create_input("a  b  ", 3), ["a", "b"]);
        assert!(split_create_input("   ", 2).is_empty());
    }

    #[test]
    fn create_prompts_need_a_scope() {
        let prompt = |tab, cat, sch| match create_prompt(tab, cat, sch) {
            CreateForm::Prompt(text) => format!("prompt: {text}"),
            CreateForm::Unsupported(text) => format!("unsupported: {text}"),
            CreateForm::ScopeMissing(text) => format!("scope: {text}"),
        };
        let (c, s) = (Some("c"), Some("s"));
        assert!(prompt(Tab::Catalogs, None, None).starts_with("prompt: New catalog"));
        assert!(prompt(Tab::Schemas, c, None).contains("under `c`"));
        assert!(prompt(Tab::Schemas, None, None).starts_with("scope: "));
        assert!(prompt(Tab::Volumes, c, s).contains("under `c.s`"));
        assert!(prompt(Tab::Volumes, c, None).starts_with("scope: "));
        assert!(prompt(Tab::Models, c, s).contains("under `c.s`"));
        assert!(prompt(Tab::Models, None, None).starts_with("scope: "));
        for tab in [
            Tab::Credentials,
            Tab::ExternalLocations,
            Tab::Tokens,
            Tab::ScimUsers,
        ] {
            assert!(prompt(tab, None, None).starts_with("prompt: "));
        }
        for tab in [Tab::Tables, Tab::Functions, Tab::Namespaces] {
            assert!(prompt(tab, c, s).starts_with("unsupported: "));
        }
    }

    #[test]
    fn list_titles_show_the_scope() {
        let hint = |tab, cat, sch| current_scope_hint(tab, cat, sch).map(Cow::into_owned);
        assert_eq!(hint(Tab::Tables, Some("c"), Some("s")).unwrap(), "c.s");
        assert!(
            hint(Tab::Volumes, Some("c"), None)
                .unwrap()
                .starts_with('⚠')
        );
        assert_eq!(hint(Tab::Schemas, Some("c"), None).unwrap(), "c");
        assert!(hint(Tab::Schemas, None, None).unwrap().starts_with('⚠'));
        assert_eq!(hint(Tab::Catalogs, Some("c"), Some("s")), None);
    }

    #[test]
    fn names_and_scopes() {
        assert_eq!(three_part_name(Some("c"), Some("s"), Some("t")), "c.s.t");
        assert_eq!(three_part_name(None, Some("s"), Some("t")), "t");
        assert_eq!(three_part_name(Some("c"), Some("s"), None), "(unnamed)");
        assert_eq!(scope_or_err(Some("c"), Some("s")), Ok(("c", "s")));
        assert!(scope_or_err(Some("c"), None).is_err());
    }

    #[test]
    fn modals_keep_a_minimum_size_inside_the_screen() {
        let screen = Rect::new(0, 0, 100, 50);
        assert_eq!(
            centered_rect(50, 20, 10, 5, screen),
            Rect::new(25, 20, 50, 10)
        );
        assert_eq!(
            centered_rect(10, 10, 40, 8, screen),
            Rect::new(30, 21, 40, 8)
        );
        let small = Rect::new(2, 3, 20, 4);
        assert_eq!(centered_rect(50, 50, 40, 8, small), small);
    }

    #[test]
    fn panes_select_in_a_ring() {
        let mut pane = Pane::default();
        pane.select_next();
        pane.select_prev();
        assert!(pane.selected().is_none());
        pane.set_rows(vec![row("a"), row("b"), row("c")]);
        assert_eq!(pane.selected().unwrap().label, "a");
        pane.select_prev();
        assert_eq!(pane.selected().unwrap().label, "c");
        pane.select_next();
        assert_eq!(pane.selected().unwrap().label, "a");
        pane.select_next();
        assert_eq!(pane.selected().unwrap().label, "b");
        pane.detail_scroll = 20;
        pane.set_error("no".into());
        assert!(pane.rows.is_empty() && pane.selected().is_none());
        assert_eq!(pane.error.as_deref(), Some("no"));
        pane.set_rows(vec![row("a")]);
        assert_eq!((pane.error.as_deref(), pane.detail_scroll), (None, 0));
    }

    #[test]
    fn the_state_file_is_under_the_state_folder() {
        if let Some(path) = state_path() {
            assert!(path.ends_with("uc/state.json"), "{}", path.display());
        }
    }

    #[tokio::test]
    async fn browses_every_tab() {
        let origin = server().await;
        let api = client(&origin);
        let dir = tempfile::tempdir().unwrap();
        let volume = external_folder(dir.path());
        managed_catalog(&api, dir.path()).await;
        for (tab, input, cat, sch) in [
            (Tab::Schemas, "s", Some("c"), None),
            (
                Tab::Volumes,
                &*format!("v EXTERNAL {volume}"),
                Some("c"),
                Some("s"),
            ),
            (Tab::Models, "m", Some("c"), Some("s")),
            (
                Tab::Credentials,
                "cred STORAGE arn:aws:iam::123456789012:role/r",
                None,
                None,
            ),
            (
                Tab::ExternalLocations,
                "loc s3://bucket/path cred",
                None,
                None,
            ),
            (Tab::ScimUsers, "ada@example.com Ada", None, None),
            (Tab::Tokens, "pat 30", None, None),
        ] {
            create_row(&api, tab, input, cat, sch).await.unwrap();
        }
        external_table(&api).await;
        python_function(&api).await;

        let mut app = app(&origin);
        app.refresh_current();
        assert!(app.pane().loading);
        assert!(shows(&screen(&mut app, 120, 30), "loading…"));
        settle(&mut app).await;
        assert_eq!(labels(&app), ["c"]);
        assert_eq!(status(&app), "loaded 1 Catalogs");
        let shown = screen(&mut app, 120, 30);
        assert!(shows(&shown, "▶ c"), "{shown:#?}");
        assert!(shows(&shown, r#""name": "c""#), "{shown:#?}");

        // Enter goes from a catalog to its schemas, then to the tables.
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.tab, Tab::Schemas);
        assert_eq!(status(&app), "scope: catalog=c");
        settle(&mut app).await;
        assert_eq!(labels(&app), ["c.s"]);
        assert!(shows(&screen(&mut app, 120, 30), " Schemas · c "));
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.tab, Tab::Tables);
        assert_eq!(status(&app), "scope: schema=c.s");
        settle(&mut app).await;
        assert_eq!(labels(&app), ["c.s.t"]);
        assert!(shows(&screen(&mut app, 120, 30), " Tables · c.s "));
        // Elsewhere, Enter does nothing.
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.tab, Tab::Tables);

        for (tab, label) in [
            (Tab::Volumes, "c.s.v".to_string()),
            (Tab::Functions, "c.s.f".into()),
            (Tab::Models, "c.s.m".into()),
            (Tab::Credentials, "cred".into()),
            (Tab::ExternalLocations, "loc  →  s3://bucket/path".into()),
            (Tab::Namespaces, "c.s".into()),
        ] {
            go_to(&mut app, tab).await;
            assert_eq!(labels(&app), [label.as_str()], "{tab:?}");
        }
        go_to(&mut app, Tab::ScimUsers).await;
        assert_eq!(app.pane().rows.len(), 1);
        assert!(labels(&app)[0].starts_with("ada@example.com  ("));
        go_to(&mut app, Tab::Tokens).await;
        assert!(
            labels(&app)[0].starts_with("pat  expires "),
            "{:?}",
            labels(&app)
        );

        // Round to the start, and back.
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.tab, Tab::Catalogs);
        settle(&mut app).await;
        press(&mut app, KeyCode::BackTab);
        assert_eq!(app.tab, Tab::Tokens);
        settle(&mut app).await;
        press(&mut app, KeyCode::Left);
        assert_eq!(app.tab, Tab::ScimUsers);
        settle(&mut app).await;

        // Backspace leaves the schema, then the catalog.
        press(&mut app, KeyCode::Backspace);
        settle(&mut app).await;
        assert_eq!(
            (
                app.current_catalog.as_deref(),
                app.current_schema.as_deref()
            ),
            (Some("c"), None)
        );
        assert_eq!(status(&app), "loaded 1 ScimUsers");
        press(&mut app, KeyCode::Backspace);
        settle(&mut app).await;
        assert_eq!(app.current_catalog, None);
        press(&mut app, KeyCode::Backspace);
        settle(&mut app).await;
        assert_eq!(app.current_catalog, None);
    }

    #[tokio::test]
    async fn tabs_without_their_scope_say_what_to_pick() {
        let origin = server().await;
        let mut app = app(&origin);
        for (tab, error) in [
            (
                Tab::Schemas,
                "select a catalog first (Catalogs tab + Enter)",
            ),
            (
                Tab::Tables,
                "select a schema first (Catalogs → Enter, then Schemas → Enter)",
            ),
            (
                Tab::Volumes,
                "select a schema first (Catalogs → Enter, then Schemas → Enter)",
            ),
            (
                Tab::Functions,
                "select a schema first (Catalogs → Enter, then Schemas → Enter)",
            ),
            (
                Tab::Namespaces,
                "select a catalog first (Catalogs tab + Enter)",
            ),
        ] {
            go_to(&mut app, tab).await;
            assert_eq!(app.pane().error.as_deref(), Some(error), "{tab:?}");
            assert!(shows(&screen(&mut app, 140, 30), &error[..20]), "{tab:?}");
        }

        for (tab, message) in [
            (
                Tab::Schemas,
                "select a catalog first (Catalogs tab → Enter)",
            ),
            (
                Tab::Volumes,
                "select a schema first (Catalogs → Enter → Schemas → Enter)",
            ),
            (
                Tab::Models,
                "select a schema first (Catalogs → Enter → Schemas → Enter)",
            ),
            (
                Tab::Tables,
                "tables need a column schema — use `uc table` from a shell",
            ),
            (
                Tab::Functions,
                "functions need a parameter signature — use `uc function` from a shell",
            ),
            (
                Tab::Namespaces,
                "Iceberg namespaces are UC schemas — create one in the Schemas tab",
            ),
        ] {
            go_to(&mut app, tab).await;
            press(&mut app, KeyCode::Char('c'));
            assert!(matches!(app.modal, Modal::None), "{tab:?}");
            assert_eq!(status(&app), message);
        }

        // The create and delete calls refuse what the keys never send.
        let api = client(&origin);
        for tab in [Tab::Tables, Tab::Functions, Tab::Namespaces] {
            assert!(
                create_row(&api, tab, "x", Some("c"), Some("s"))
                    .await
                    .is_err()
            );
        }
        for tab in [Tab::Schemas, Tab::Volumes, Tab::Models] {
            assert!(create_row(&api, tab, "x", None, None).await.is_err());
        }
        assert_eq!(
            delete_row(&api, Tab::Namespaces, "c.s", false)
                .await
                .err()
                .map(|failure| failure.text)
                .as_deref(),
            Some("drop_namespace not supported from TUI")
        );
    }

    #[tokio::test]
    async fn creates_and_deletes_on_every_tab() {
        let origin = server().await;
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(&origin);
        app.refresh_current();
        settle(&mut app).await;

        // The input box: typing, Backspace, Esc and nothing entered.
        press(&mut app, KeyCode::Char('c'));
        type_text(&mut app, "ab");
        press(&mut app, KeyCode::Backspace);
        press(&mut app, KeyCode::Down);
        let Modal::TextInput { value, .. } = &app.modal else {
            panic!("no input box")
        };
        assert_eq!(value, "a");
        assert!(shows(&screen(&mut app, 120, 30), "▸ a_"));
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.modal, Modal::None));
        assert!(!app.should_quit);
        press(&mut app, KeyCode::Char('c'));
        press(&mut app, KeyCode::Enter);
        assert_eq!(status(&app), "nothing entered — create cancelled");

        assert_eq!(
            create(&mut app, "made the catalog").await,
            "created catalog 'made'"
        );
        assert_eq!(app.pane().rows[0].detail["comment"], "the catalog");
        let refused = create(&mut app, "made").await;
        assert!(refused.contains("already exists"), "{refused}");
        // Models need managed storage, which a catalog made here has not.
        managed_catalog(&client(&origin), dir.path()).await;
        press(&mut app, KeyCode::Char('r'));
        settle(&mut app).await;
        assert_eq!(labels(&app), ["c", "made"]);
        press(&mut app, KeyCode::Enter);
        settle(&mut app).await;
        assert_eq!(create(&mut app, "s").await, "created schema 'c.s'");
        press(&mut app, KeyCode::Enter);
        settle(&mut app).await;

        go_to(&mut app, Tab::Volumes).await;
        let refused = create(&mut app, "v SOMETIMES").await;
        assert_eq!(
            refused,
            "volume_type must be MANAGED or EXTERNAL, got 'SOMETIMES'"
        );
        let input = format!("v EXTERNAL {}", external_folder(dir.path()));
        assert_eq!(create(&mut app, &input).await, "created volume 'c.s.v'");
        go_to(&mut app, Tab::Models).await;
        assert_eq!(create(&mut app, "m a model").await, "created model 'c.s.m'");
        go_to(&mut app, Tab::Credentials).await;
        let refused = create(&mut app, "cred SOMETHING").await;
        assert_eq!(
            refused,
            "purpose must be STORAGE or SERVICE_PRINCIPAL, got 'SOMETHING'"
        );
        let input = "cred STORAGE arn:aws:iam::123456789012:role/r";
        assert_eq!(create(&mut app, input).await, "created credential 'cred'");
        go_to(&mut app, Tab::ExternalLocations).await;
        let refused = create(&mut app, "loc s3://bucket/path").await;
        assert_eq!(refused, "expected `<name> <url> <credential_name>`");
        let input = "loc s3://bucket/path cred";
        assert_eq!(
            create(&mut app, input).await,
            "created external_location 'loc'"
        );
        go_to(&mut app, Tab::ScimUsers).await;
        let created = create(&mut app, "bob@example.com Bob B").await;
        assert!(
            created.starts_with("created SCIM user 'bob@example.com' (id "),
            "{created}"
        );
        assert_eq!(app.pane().rows[0].detail["displayName"], "Bob B");

        // After the name, the expiry: 30 days unless changed. A new token
        // is shown once, until Enter or Esc.
        go_to(&mut app, Tab::Tokens).await;
        press(&mut app, KeyCode::Char('c'));
        type_text(&mut app, "pat");
        press(&mut app, KeyCode::Enter);
        let Modal::TextInput { prompt, value, .. } = &app.modal else {
            panic!("no expiry question: {}", status(&app))
        };
        assert!(prompt.starts_with("Expiry — days"), "{prompt}");
        assert_eq!(value, "30");
        press(&mut app, KeyCode::Enter);
        settle(&mut app).await;
        let Modal::Secret { title, secret, .. } = &app.modal else {
            panic!("no secret: {}", status(&app))
        };
        assert_eq!(title, "PAT 'pat' created");
        assert!(secret.len() > 8);
        let secret = secret.clone();
        let shown = screen(&mut app, 120, 30);
        assert!(shows(&shown, " Secret "), "{shown:#?}");
        assert!(shows(&shown, &secret), "{shown:#?}");
        press(&mut app, KeyCode::Char('q'));
        assert!(matches!(app.modal, Modal::Secret { .. }));
        assert!(!app.should_quit);
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.modal, Modal::None));
        settle(&mut app).await;
        assert_eq!(app.pane().rows.len(), 1);
        let expires_at = app.pane().rows[0].detail["expires_at"].as_i64().unwrap();
        let days = (expires_at - Local::now().timestamp_millis()) as f64 / 86_400_000.0;
        assert!((29.9..=30.0).contains(&days), "{days}");

        // `n` and Esc keep the row.
        for key in [KeyCode::Char('n'), KeyCode::Esc] {
            press(&mut app, KeyCode::Char('d'));
            let Modal::Confirm { prompt, .. } = &app.modal else {
                panic!("no confirmation")
            };
            let id = app.pane().rows[0].delete_id.clone().unwrap();
            assert_eq!(prompt, &format!("Delete Tokens '{id}'?  (y / n)"));
            assert!(shows(&screen(&mut app, 120, 30), " Confirm "));
            press(&mut app, KeyCode::Char('x'));
            assert!(matches!(app.modal, Modal::Confirm { .. }));
            press(&mut app, key);
            assert!(matches!(app.modal, Modal::None));
        }
        assert_eq!(app.pane().rows.len(), 1);

        let id = app.pane().rows[0].delete_id.clone().unwrap();
        assert_eq!(delete(&mut app).await, format!("deleted Tokens '{id}'"));
        assert!(app.pane().rows.is_empty());
        // Nothing to delete.
        press(&mut app, KeyCode::Char('d'));
        assert!(matches!(app.modal, Modal::None));

        go_to(&mut app, Tab::ScimUsers).await;
        assert!(delete(&mut app).await.starts_with("deleted ScimUsers '"));
        go_to(&mut app, Tab::Namespaces).await;
        press(&mut app, KeyCode::Char('d'));
        assert_eq!(status(&app), "delete not supported on Namespaces");

        // A catalog with schemas is not deleted: the TUI says why and what it
        // holds, and asks whether to delete it all.
        go_to(&mut app, Tab::Catalogs).await;
        assert_eq!(app.pane().selected().unwrap().label, "c");
        let refused = delete(&mut app).await;
        assert!(
            refused.starts_with("Catalogs 'c' not deleted: "),
            "{refused}"
        );
        let Modal::Confirm { prompt, .. } = &app.modal else {
            panic!("no question: {refused}")
        };
        assert!(
            prompt.starts_with("Cannot delete catalog with schemas."),
            "{prompt}"
        );
        assert!(
            prompt.ends_with("\n\nc holds 1 schema, 1 volume and 1 registered model."),
            "{prompt}"
        );
        assert!(shows(&screen(&mut app, 120, 30), " Not empty "));
        // Only `f` goes on; `n` keeps the catalog.
        for key in [KeyCode::Char('y'), KeyCode::Enter] {
            press(&mut app, key);
            assert!(matches!(app.modal, Modal::Confirm { .. }));
        }
        press(&mut app, KeyCode::Char('n'));
        assert!(matches!(app.modal, Modal::None));
        assert_eq!(labels(&app), ["c", "made"]);

        let api = client(&origin);
        external_table(&api).await;
        python_function(&api).await;
        for (tab, name) in [
            (Tab::Tables, "c.s.t"),
            (Tab::Volumes, "c.s.v"),
            (Tab::Functions, "c.s.f"),
            (Tab::Models, "c.s.m"),
            (Tab::ExternalLocations, "loc"),
            (Tab::Credentials, "cred"),
            (Tab::Schemas, "c.s"),
            (Tab::Catalogs, "c"),
            (Tab::Catalogs, "made"),
        ] {
            go_to(&mut app, tab).await;
            assert_eq!(
                delete(&mut app).await,
                format!("deleted {} '{name}'", tab.title())
            );
        }
        assert!(app.pane().rows.is_empty());
    }

    #[tokio::test]
    async fn a_catalog_goes_with_all_it_holds_after_two_more_questions() {
        let origin = server().await;
        let api = client(&origin);
        api.create_catalog("c").await.unwrap();
        api.create_schema("s", "c").await.unwrap();
        external_table(&api).await;
        let mut app = app(&origin);
        app.refresh_current();
        settle(&mut app).await;
        assert_eq!(labels(&app), ["c"]);

        assert!(
            delete(&mut app)
                .await
                .starts_with("Catalogs 'c' not deleted: ")
        );
        let shown = screen(&mut app, 120, 30);
        assert!(shows(&shown, "c holds 1 schema and 1 table."), "{shown:#?}");
        assert!(shows(&shown, "f = also delete all it holds"), "{shown:#?}");
        // Red, as a failure.
        let failed = |app: &App| app.failure.is_some() && app.failure == app.status_msg;
        assert!(failed(&app));

        // The last question names what goes; only `y` answers it.
        press(&mut app, KeyCode::Char('f'));
        let Modal::Confirm { prompt, .. } = &app.modal else {
            panic!("no last question")
        };
        assert_eq!(
            prompt,
            "This deletes the catalog c and all it holds: 1 schema and 1 table. The data of \
             managed tables and volumes, and of models, goes too.\n\nThis operation cannot be \
             undone."
        );
        let shown = screen(&mut app, 120, 30);
        assert!(shows(&shown, " Are you sure? "), "{shown:#?}");
        assert!(
            shows(&shown, "This operation cannot be undone."),
            "{shown:#?}"
        );
        assert!(shows(&shown, "y = delete everything"), "{shown:#?}");
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.modal, Modal::Confirm { .. }));
        press(&mut app, KeyCode::Char('y'));
        assert_eq!(status(&app), "deleting Catalogs 'c'…");
        settle(&mut app).await;
        assert_eq!(status(&app), "deleted Catalogs 'c' and all it held");
        assert!(!failed(&app));
        settle(&mut app).await;
        assert!(app.pane().rows.is_empty());
        assert!(api.list_schemas("c").await.is_err());
    }

    #[tokio::test]
    async fn keys_move_scroll_and_quit() {
        let mut app = offline();
        app.pane_mut().set_rows(vec![row("a"), row("b"), row("c")]);
        for (key, label) in [
            (KeyCode::Down, "b"),
            (KeyCode::Char('j'), "c"),
            (KeyCode::Up, "b"),
            (KeyCode::Char('k'), "a"),
        ] {
            press(&mut app, key);
            assert_eq!(app.pane().selected().unwrap().label, label);
        }
        press(&mut app, KeyCode::PageDown);
        press(&mut app, KeyCode::Char(' '));
        assert_eq!(app.pane().detail_scroll, 2 * DETAIL_SCROLL_STEP);
        press(&mut app, KeyCode::PageUp);
        assert_eq!(app.pane().detail_scroll, DETAIL_SCROLL_STEP);
        // Shift-↓ and Shift-↑ scroll by a line, and keep the row.
        let shift = |code| Event::Key(KeyEvent::new(code, KeyModifiers::SHIFT));
        app.handle_event(shift(KeyCode::Down));
        assert_eq!(app.pane().detail_scroll, DETAIL_SCROLL_STEP + 1);
        app.handle_event(shift(KeyCode::Up));
        app.handle_event(shift(KeyCode::Up));
        assert_eq!(app.pane().detail_scroll, DETAIL_SCROLL_STEP - 1);
        assert_eq!(app.pane().selected().unwrap().label, "a");
        press(&mut app, KeyCode::Down);
        assert_eq!(app.pane().detail_scroll, 0);

        // Only presses count.
        app.handle_event(Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        )));
        app.handle_event(Event::FocusGained);
        press(&mut app, KeyCode::F(5));
        assert!(!app.should_quit);

        // `?` shows the keys; any key closes them.
        press(&mut app, KeyCode::Char('?'));
        assert!(matches!(app.modal, Modal::Help));
        assert!(shows(&screen(&mut app, 120, 30), "uc TUI — keys"));
        // All of it, on a small screen too.
        let small = screen(&mut app, 80, 24);
        assert!(shows(&small, "also delete all it holds"), "{small:#?}");
        assert!(shows(&small, "any key closes this overlay"), "{small:#?}");
        press(&mut app, KeyCode::Char('q'));
        assert!(matches!(app.modal, Modal::None));
        assert!(!app.should_quit);

        // `r` reloads.
        press(&mut app, KeyCode::Char('r'));
        assert!(app.pane().loading);

        press(&mut app, KeyCode::Char('q'));
        assert!(app.should_quit);
        let mut app = offline();
        press(&mut app, KeyCode::Esc);
        assert!(app.should_quit);
        // Ctrl-C quits, also from a box.
        let mut app = offline();
        press(&mut app, KeyCode::Char('s'));
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )));
        assert!(app.should_quit);
    }

    #[test]
    fn the_detail_stops_at_its_end_and_has_a_scroll_bar() {
        let mut app = offline();
        app.pane_mut().set_rows(vec![row("short"), row("long")]);
        // 42 lines: `{`, key00 to key39, and `}`.
        let long: serde_json::Map<String, serde_json::Value> =
            (0..40).map(|i| (format!("key{i:02}"), json!(i))).collect();
        app.pane_mut().rows[1].detail = serde_json::Value::Object(long);
        // The right line of the detail, between its corners.
        let bar = |app: &mut App| {
            let column: String = screen(app, 140, 30)
                .iter()
                .filter_map(|line| line.chars().last())
                .collect();
            let start = column.rfind('┐').unwrap() + '┐'.len_utf8();
            let end = column.rfind('┘').unwrap();
            column[start..end].to_string()
        };

        // A detail that fits has no scroll bar.
        assert!(!bar(&mut app).contains('┃'));

        // A long one has its thumb at the top, then, a line at a time, at
        // the bottom at the end. The thumb keeps its size on the way.
        press(&mut app, KeyCode::Down);
        let top = bar(&mut app);
        assert!(top.starts_with('┃') && top.ends_with('│'), "{top}");
        let shift = |code| Event::Key(KeyEvent::new(code, KeyModifiers::SHIFT));
        for _ in 0..30 {
            app.handle_event(shift(KeyCode::Down));
            let bar = bar(&mut app);
            assert_eq!(bar.matches('┃').count(), top.matches('┃').count(), "{bar}");
        }
        let end = bar(&mut app);
        assert!(end.starts_with('│') && end.ends_with('┃'), "{end}");

        // The scroll stops with the last line at the bottom, so that
        // Shift-↑ moves the detail at once.
        let last = app.pane().detail_scroll;
        for _ in 0..10 {
            press(&mut app, KeyCode::PageDown);
        }
        let shown = screen(&mut app, 140, 30);
        assert!(shows(&shown, "key39"), "{shown:#?}");
        assert_eq!(app.pane().detail_scroll, last);
        app.handle_event(shift(KeyCode::Up));
        assert_eq!(app.pane().detail_scroll, last - 1);
    }

    #[tokio::test]
    async fn the_server_can_be_changed() {
        let origin = server().await;
        let mut app = offline();
        app.refresh_current();
        settle(&mut app).await;
        assert!(app.pane().error.is_some());

        // The box starts with the server.
        press(&mut app, KeyCode::Char('s'));
        let Modal::TextInput { value, .. } = &app.modal else {
            panic!("no input box")
        };
        assert_eq!(value, &app.server_url);
        let typed = value.chars().count();
        for _ in 0..typed {
            press(&mut app, KeyCode::Backspace);
        }
        press(&mut app, KeyCode::Enter);
        assert_eq!(status(&app), "server URL must not be empty");

        press(&mut app, KeyCode::Char('s'));
        for _ in 0..typed {
            press(&mut app, KeyCode::Backspace);
        }
        type_text(&mut app, "not a url");
        press(&mut app, KeyCode::Enter);
        assert!(status(&app).starts_with("bad URL: "), "{}", status(&app));

        // The server's tabs are loaded again; no token goes to a new server.
        app.panes[Tab::Tokens.index()].set_rows(vec![row("old")]);
        press(&mut app, KeyCode::Char('s'));
        for _ in 0..typed {
            press(&mut app, KeyCode::Backspace);
        }
        let url = format!("{origin}{API_BASE}/");
        type_text(&mut app, &url);
        press(&mut app, KeyCode::Enter);
        assert_eq!(status(&app), "server changed — refetching");
        assert_eq!(app.server_url, url);
        assert!(app.panes[Tab::Tokens.index()].rows.is_empty());
        settle(&mut app).await;
        assert_eq!(status(&app), "loaded 0 Catalogs");
        assert!(shows(&screen(&mut app, 120, 30), "(empty)"));
    }

    #[tokio::test]
    async fn late_and_failed_results() {
        let mut app = offline();
        // A fetch that a newer one replaced is dropped.
        app.fetch_gen[Tab::Catalogs.index()] = 2;
        app.apply_fetch_result(FetchResult {
            tab: Tab::Catalogs,
            generation: 1,
            rows: Ok(vec![row("old")]),
        });
        assert!(app.pane().rows.is_empty());
        // Another tab's rows load without changing the status.
        app.fetch_gen[Tab::Schemas.index()] = 1;
        app.apply_fetch_result(FetchResult {
            tab: Tab::Schemas,
            generation: 1,
            rows: Ok(vec![row("c.s")]),
        });
        assert_eq!(app.panes[Tab::Schemas.index()].rows.len(), 1);
        assert_eq!(app.status_msg, None);
        app.fetch_gen[Tab::Catalogs.index()] = 3;
        app.apply_fetch_result(FetchResult {
            tab: Tab::Catalogs,
            generation: 3,
            rows: Err("down".into()),
        });
        assert_eq!(app.pane().error.as_deref(), Some("down"));

        app.apply_mutation_result(MutationResult {
            tab: Tab::Catalogs,
            outcome: Err("create failed".into()),
        });
        assert_eq!(status(&app), "create failed");
        // Another tab's change does not reload this one.
        app.apply_mutation_result(MutationResult {
            tab: Tab::Schemas,
            outcome: Ok(MutationOutcome::Message("deleted".into())),
        });
        assert_eq!(status(&app), "deleted");
        assert!(!app.pane().loading);
    }

    /// The server's 401.
    fn unauthenticated() -> Failure {
        let body = json!({ "error_code": "UNAUTHENTICATED", "message": "No authorization found." });
        ClientError::Api(ApiError::from_body(401, body.to_string().as_bytes())).into()
    }

    #[tokio::test]
    async fn the_tui_follows_the_sign_in() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("tokens.json");
        let login = "uc --config x.toml auth login".to_string();
        let mut app = offline().with_sign_ins(Some(SignIns::new(&file)), None, login.clone());

        // A 401 says how to sign in.
        app.fetch_gen[Tab::Catalogs.index()] = 1;
        app.apply_fetch_result(FetchResult {
            tab: Tab::Catalogs,
            generation: 1,
            rows: Err(unauthenticated()),
        });
        assert_eq!(
            app.pane().error.as_deref(),
            Some(
                "No authorization found.\n\nhttp://127.0.0.1:9 needs a sign-in. Sign in with: uc \
                 --config x.toml auth login\nThen press r."
            )
        );
        app.apply_mutation_result(MutationResult {
            tab: Tab::Catalogs,
            outcome: Err(unauthenticated()),
        });
        assert_eq!(
            status(&app),
            "No authorization found. http://127.0.0.1:9 needs a sign-in. Sign in with: uc --config \
             x.toml auth login"
        );

        // A sign-in in another terminal counts from the next fetch.
        SignIns::new(&file)
            .save("http://127.0.0.1:9", "signed-in")
            .unwrap();
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.token, Some(("signed-in".into(), TokenSource::SignIn)));
        SignIns::new(&file).remove("http://127.0.0.1:9").unwrap();
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.token, None);

        // A given token stays, and a 401 names it.
        let given = Some(("given".to_string(), TokenSource::Option));
        let mut app = offline().with_sign_ins(Some(SignIns::new(&file)), given.clone(), login);
        SignIns::new(&file)
            .save("http://127.0.0.1:9", "signed-in")
            .unwrap();
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.token, given);
        app.fetch_gen[Tab::Catalogs.index()] = 1;
        app.apply_fetch_result(FetchResult {
            tab: Tab::Catalogs,
            generation: 1,
            rows: Err(unauthenticated()),
        });
        let error = app.pane().error.clone().unwrap();
        assert!(error.contains("the token of --auth_token."), "{error}");
        assert!(!error.contains("press r"), "{error}");

        // Another server gets its own sign-in, not the given token.
        SignIns::new(&file)
            .save("http://127.0.0.1:10", "other")
            .unwrap();
        press(&mut app, KeyCode::Char('s'));
        for _ in 0..app.server_url.chars().count() {
            press(&mut app, KeyCode::Backspace);
        }
        type_text(&mut app, &format!("http://127.0.0.1:10{API_BASE}/"));
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.token, Some(("other".into(), TokenSource::SignIn)));
    }

    #[test]
    fn the_tab_bar_wraps_between_titles() {
        let text = |lines: Vec<Line>| lines.iter().map(Line::to_string).collect::<Vec<_>>();
        let titles = ["Catalogs", "Schemas", "Tables"];
        assert_eq!(
            text(tab_lines(&titles, 0, 40)),
            [" Catalogs │ Schemas │ Tables "]
        );
        // " Catalogs │ Schemas " has 20 columns.
        assert_eq!(
            text(tab_lines(&titles, 0, 20)),
            [" Catalogs │ Schemas ", " Tables "]
        );
        assert_eq!(
            text(tab_lines(&titles, 0, 19)),
            [" Catalogs ", " Schemas │ Tables "]
        );
        // A title wider than the line is alone on its line.
        assert_eq!(
            text(tab_lines(&titles, 0, 5)),
            [" Catalogs ", " Schemas ", " Tables "]
        );
        // Only the selected title is highlighted, with its spaces.
        let lines = tab_lines(&titles, 1, 40);
        let highlighted: String = lines[0]
            .spans
            .iter()
            .filter(|span| span.style.bg == Some(colors::PRIMARY))
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(highlighted, " Schemas ");
    }

    #[test]
    fn the_screen_has_the_web_uis_colors() {
        let mut app = offline();
        app.pane_mut().set_rows(vec![row("alpha"), row("beta")]);
        app.status_msg = Some("loaded 2 Catalogs".into());
        let colors_at = |buffer: &Buffer, text| {
            let cell = &buffer[position(buffer, text)];
            (cell.fg, cell.bg)
        };

        // The header: the selected tab white on blue, the others light on
        // navy. The page: dark text on white, the selected row light blue.
        let shown = buffer(&mut app, 140, 30);
        assert_eq!(
            colors_at(&shown, "Catalogs"),
            (colors::WHITE, colors::PRIMARY)
        );
        assert_eq!(
            colors_at(&shown, "Schemas"),
            (colors::HEADER_TEXT, colors::HEADER)
        );
        assert_eq!(colors_at(&shown, "beta"), (colors::TEXT, colors::PAGE));
        assert_eq!(colors_at(&shown, "alpha"), (colors::TEXT, colors::SELECTED));
        assert_eq!(colors_at(&shown, "loaded").0, colors::SUCCESS);
        assert_eq!(colors_at(&shown, "? help").0, colors::SECONDARY);
        app.failure = app.status_msg.clone();
        assert_eq!(
            colors_at(&buffer(&mut app, 140, 30), "loaded").0,
            colors::DANGER
        );

        // A dialog is white over the screen, which goes darker.
        app.modal = Modal::Confirm {
            prompt: "Delete catalog alpha?".into(),
            action: PendingAction::Delete {
                tab: Tab::Catalogs,
                id: "alpha".into(),
                force: false,
            },
        };
        let shown = buffer(&mut app, 140, 30);
        assert_eq!(
            colors_at(&shown, "Delete catalog"),
            (colors::TEXT, colors::PAGE)
        );
        assert_eq!(colors_at(&shown, "Confirm").0, colors::DANGER);
        assert_eq!(colors_at(&shown, "y = confirm").0, colors::DANGER);
        assert_eq!(colors_at(&shown, "n / Esc").0, colors::SECONDARY);
        let dark = Color::Rgb(0x8c, 0x8c, 0x8c);
        assert_eq!(
            colors_at(&shown, "beta"),
            (Color::Rgb(0x11, 0x11, 0x11), dark)
        );

        // Without 24-bit colors, the nearest of the 256.
        app.true_color = false;
        app.modal = Modal::None;
        let shown = buffer(&mut app, 140, 30);
        assert_eq!(
            colors_at(&shown, "Catalogs"),
            (Color::Indexed(231), Color::Indexed(33))
        );
        // Each cell that the terminal draws: not the one behind the second
        // column of the crab in the title, which ratatui does not send.
        let cells = &shown.content;
        let all_256 = cells.iter().enumerate().all(|(i, cell)| {
            let hidden = i > 0 && Span::raw(cells[i - 1].symbol()).width() > 1;
            hidden || matches!((cell.fg, cell.bg), (Color::Indexed(_), Color::Indexed(_)))
        });
        assert!(all_256);

        // A list that failed to load: the web UI's error box.
        app.true_color = true;
        app.pane_mut().set_error("No authorization found.".into());
        assert_eq!(
            colors_at(&buffer(&mut app, 140, 30), "No authorization"),
            (colors::TEXT, colors::ERROR_BG)
        );
    }

    #[test]
    fn colors_go_to_the_nearest_of_xterms_256() {
        let near = |color| match xterm_256(color) {
            Color::Indexed(i) => i,
            other => panic!("{other:?}"),
        };
        // The cube: 16 + 36 r + 6 g + b, with the levels 0, 95, 135, 175,
        // 215 and 255.
        assert_eq!(near(colors::PAGE), 231);
        assert_eq!(near(Color::Rgb(0, 0, 0)), 16);
        assert_eq!(near(colors::PRIMARY), 33);
        assert_eq!(near(colors::DANGER), 203);
        assert_eq!(near(colors::SUCCESS), 76);
        assert_eq!(near(colors::SELECTED), 195);
        // The greys: 232 + i for 8 + 10 i.
        assert_eq!(near(colors::TEXT), 234);
        assert_eq!(near(colors::SECONDARY), 245);
        assert_eq!(near(colors::BORDER), 253);
        assert_eq!(near(colors::HEADER), 233);
        assert_eq!(xterm_256(Color::Reset), Color::Reset);
    }

    #[test]
    fn the_detail_shows_the_logo_without_a_row() {
        let mut app = offline();
        let logo = |app: &mut App| shows(&screen(app, 120, 30), "▀");
        assert!(logo(&mut app));
        app.pane_mut().set_rows(vec![row("alpha")]);
        assert!(!logo(&mut app));
    }

    #[test]
    fn layouts_follow_the_screen_size() {
        let mut app = offline();
        app.pane_mut().set_rows(vec![row("alpha"), row("beta")]);
        app.status_msg = Some("loaded 2 Catalogs".into());

        let small = screen(&mut app, 39, 10);
        assert!(shows(&small, "uc TUI needs at least 40×8"), "{small:#?}");
        assert!(shows(&screen(&mut app, 80, 7), "current window is 80×7."));

        // Wide: titles, the whole keymap, list and detail side by side.
        let wide = screen(&mut app, 140, 20);
        // The crab takes two columns: the second one shows as a space here.
        assert!(shows(&wide, " 🦀  uc · Unity Catalog "), "{wide:#?}");
        assert!(shows(&wide, "ExternalLocs"), "{wide:#?}");
        assert!(shows(&wide, "⏎ drill · ⌫ scope"), "{wide:#?}");
        assert!(shows(&wide, "loaded 2 Catalogs"), "{wide:#?}");
        assert!(shows(
            &screen(&mut app, 200, 20),
            "r refresh · ? help · q quit"
        ));
        // The keymap goes on to the next line, and so do the titles, where
        // they do not fit on one.
        assert!(shows(&wide, "? help · q quit"), "{wide:#?}");
        let two_lines = screen(&mut app, 110, 20);
        assert!(shows(&two_lines, " Namespaces "), "{two_lines:#?}");
        assert!(shows(&two_lines, "│ ScimUsers │ Tokens "), "{two_lines:#?}");
        assert!(shows(&two_lines, "? help · q quit"), "{two_lines:#?}");
        let side_by_side = wide
            .iter()
            .any(|l| l.contains(" Catalogs ") && l.contains(" Detail "));
        assert!(side_by_side, "{wide:#?}");
        assert!(shows(&wide, "▶ alpha"), "{wide:#?}");

        // Compact: short titles and keymap.
        let compact = screen(&mut app, 80, 20);
        assert!(!shows(&compact, "Unity Catalog"), "{compact:#?}");
        assert!(!shows(&compact, "🦀"), "{compact:#?}");
        assert!(shows(&compact, "Cat"), "{compact:#?}");
        assert!(!shows(&compact, "ExternalLocs"), "{compact:#?}");
        assert!(shows(&compact, "c create · d"), "{compact:#?}");
        assert!(shows(&compact, "delete · ? help · q quit"), "{compact:#?}");
        let wrapped = screen(&mut app, 45, 20);
        assert!(shows(&wrapped, "│ Cat │ Sch "), "{wrapped:#?}");
        assert!(shows(&wrapped, " Tok "), "{wrapped:#?}");
        assert!(shows(&wrapped, "q quit"), "{wrapped:#?}");
        // A long message takes more lines; the keys stay on screen.
        app.status_msg = Some("No authorization found. ".repeat(4));
        assert!(shows(&screen(&mut app, 45, 20), "q quit"));
        app.status_msg = Some("loaded 2 Catalogs".into());

        // Narrow: the detail under the list.
        let narrow = screen(&mut app, 50, 30);
        let list = narrow
            .iter()
            .position(|l| l.contains(" Catalogs "))
            .unwrap();
        let detail = narrow.iter().position(|l| l.contains(" Detail ")).unwrap();
        assert!(detail > list, "{narrow:#?}");

        // The detail scrolls.
        let long: serde_json::Map<String, serde_json::Value> =
            (0..40).map(|i| (format!("key{i:02}"), json!(i))).collect();
        app.pane_mut().rows[0].detail = serde_json::Value::Object(long);
        assert!(shows(&screen(&mut app, 140, 30), "key00"));
        press(&mut app, KeyCode::PageDown);
        let scrolled = screen(&mut app, 140, 30);
        assert!(!shows(&scrolled, "key00"), "{scrolled:#?}");
        assert!(shows(&scrolled, "key10"), "{scrolled:#?}");

        app.pane_mut().set_rows(Vec::new());
        assert!(shows(&screen(&mut app, 140, 20), "(empty)"));
    }

    #[tokio::test]
    async fn the_loop_draws_until_quit_and_keeps_the_tab_and_scope() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("uc/state.json");
        let mut app = App::new(
            client("http://127.0.0.1:9"),
            "http://127.0.0.1:9/".into(),
            Some(file.clone()),
        );
        // Nothing to restore yet.
        app.restore_persisted_state();
        assert_eq!(app.tab, Tab::Catalogs);
        app.current_catalog = Some("c".into());
        app.current_schema = Some("s".into());

        let key = |code| Ok(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
        let events = futures::stream::iter(vec![
            key(KeyCode::Right),
            Err(io::Error::other("a read failed")),
            key(KeyCode::Right),
            key(KeyCode::Char('q')),
        ]);
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        tokio::time::timeout(Duration::from_secs(10), app.run(&mut terminal, events))
            .await
            .expect("the loop stops on q")
            .unwrap();
        let shown: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(shown.contains(" Tables · c.s "), "{shown}");

        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        assert_eq!(
            saved,
            json!({ "tab": "Tables", "current_catalog": "c", "current_schema": "s" })
        );
        let mut app = App::new(
            client("http://127.0.0.1:9"),
            "http://127.0.0.1:9/".into(),
            Some(file.clone()),
        );
        app.restore_persisted_state();
        assert_eq!(app.tab, Tab::Tables);
        assert_eq!(app.current_catalog.as_deref(), Some("c"));
        assert_eq!(app.current_schema.as_deref(), Some("s"));

        // A file that is not the state is left alone.
        std::fs::write(&file, "not json").unwrap();
        let mut app = App::new(
            client("http://127.0.0.1:9"),
            "http://127.0.0.1:9/".into(),
            Some(file),
        );
        app.restore_persisted_state();
        assert_eq!(app.tab, Tab::Catalogs);
    }

    /// The screen of an offline app's loop at `end`, after `keys`, each at
    /// its time.
    async fn screen_at(keys: Vec<(Duration, KeyCode)>, end: Duration) -> String {
        let start = tokio::time::Instant::now();
        let events = futures::stream::iter(keys)
            .then(move |(at, code)| async move {
                tokio::time::sleep_until(start + at).await;
                Ok(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))
            })
            .chain(futures::stream::pending())
            .boxed();
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        let ran = tokio::time::timeout(end, offline().run(&mut terminal, events)).await;
        assert!(ran.is_err(), "the loop runs until the end");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[tokio::test(start_paused = true)]
    async fn the_input_cursor_blinks_and_a_key_shows_it() {
        let s = (Duration::ZERO, KeyCode::Char('s'));
        let input = format!("▸ http://127.0.0.1:9{API_BASE}/");
        let shown = screen_at(vec![s], BLINK / 2).await;
        assert!(shown.contains(&format!("{input}_")), "{shown}");
        let hidden = screen_at(vec![s], BLINK * 3 / 2).await;
        assert!(hidden.contains(&format!("{input} ")), "{hidden}");
        // Without the key, the cursor would hide again at 2 × BLINK.
        let x = (BLINK * 3 / 2, KeyCode::Char('x'));
        let typed = screen_at(vec![s, x], BLINK * 9 / 4).await;
        assert!(typed.contains(&format!("{input}x_")), "{typed}");
    }

    #[test]
    fn token_expiry_reads_days_dates_and_never() {
        let now = Local.with_ymd_and_hms(2026, 10, 16, 12, 0, 0).unwrap();
        let day = 24 * 60 * 60 * 1000;
        let in_30_days = now.timestamp_millis() + 30 * day;
        assert_eq!(token_expiry("30", now), Ok(Some(in_30_days)));
        assert_eq!(token_expiry(" Never ", now), Ok(None));
        // A date: the token works until the end of that day.
        let end = Local.with_ymd_and_hms(2026, 12, 31, 23, 59, 59).unwrap();
        let end = end.timestamp_millis() + 999;
        assert_eq!(token_expiry("2026-12-31", now), Ok(Some(end)));
        assert!(token_expiry("2026-10-16", now).unwrap().is_some());
        assert_eq!(
            token_expiry("2026-10-15", now),
            Err("the date 2026-10-15 has passed".into())
        );
        assert_eq!(
            token_expiry("0", now),
            Err("the expiry must be at least 1 day".into())
        );
        let refused = token_expiry("soon", now).unwrap_err();
        assert!(refused.starts_with("expiry must be days"), "{refused}");
    }

    #[tokio::test]
    async fn tokens_ask_for_an_expiry() {
        let origin = server().await;
        let mut app = app(&origin);
        go_to(&mut app, Tab::Tokens).await;
        // The question fits an 80-column screen.
        press(&mut app, KeyCode::Char('c'));
        type_text(&mut app, "x");
        press(&mut app, KeyCode::Enter);
        let shown = screen(&mut app, 80, 24);
        assert!(
            shows(&shown, "a date (`2026-12-31`) or `never`"),
            "{shown:#?}"
        );
        assert!(shows(&shown, "▸ 30_"), "{shown:#?}");
        press(&mut app, KeyCode::Esc);

        // `c`, the name, then the expiry in place of the offered 30 days.
        async fn create_token(app: &mut App, name: &str, expiry: &str) -> String {
            press(app, KeyCode::Char('c'));
            type_text(app, name);
            press(app, KeyCode::Enter);
            press(app, KeyCode::Backspace);
            press(app, KeyCode::Backspace);
            type_text(app, expiry);
            press(app, KeyCode::Enter);
            if !matches!(app.modal, Modal::None) {
                panic!("the expiry question is still open: {}", status(app));
            }
            if expiry.is_empty() {
                return status(app).to_string();
            }
            settle(app).await;
            if matches!(app.modal, Modal::Secret { .. }) {
                press(app, KeyCode::Enter);
                // The tab reloads.
                settle(app).await;
            }
            status(app).to_string()
        }

        let cancelled = create_token(&mut app, "x", "").await;
        assert_eq!(cancelled, "no expiry entered — create cancelled");
        let refused = create_token(&mut app, "x", "soon").await;
        assert!(refused.starts_with("expiry must be days"), "{refused}");
        let refused = create_token(&mut app, "x", "2020-01-01").await;
        assert_eq!(refused, "the date 2020-01-01 has passed");
        assert!(app.pane().rows.is_empty());

        create_token(&mut app, "nightly job", "2099-12-31").await;
        create_token(&mut app, "forever", "never").await;
        let mut shown = labels(&app);
        shown.sort();
        assert_eq!(shown.len(), 2, "{shown:?}");
        assert!(
            shown[0].starts_with("forever  never expires  ("),
            "{shown:?}"
        );
        assert!(
            shown[1].starts_with("nightly job  expires 2099-12-31 23:59  ("),
            "{shown:?}"
        );
    }
}
