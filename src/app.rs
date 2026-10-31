//! Application state, the action bus and the top-level frame layout.

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{
    Color32, CornerRadius, CursorIcon, Id, Order, Pos2, Rect, Sense, Stroke, Ui,
    Vec2, ViewportCommand,
};

use crate::dialogs::{self, Dialog, PropertiesState};
use crate::fs::{self, Drive, Entry, Location, NetworkMsg, QuickFolder, SearchMsg, Selection, SortKey};
use crate::lang::{f, self as lang, Lang, t};
use crate::settings::Preferences;
use crate::shell::{self, IconRequest, IconResponse};
use crate::theme::{self, palette_with, Accent, Palette, ThemeMode};
use crate::widgets;
use crate::{ui_chrome, ui_files, ui_nav};

pub const TAB_BAR_H: f32 = 40.0;
pub const COMMAND_BAR_H: f32 = 46.0;
pub const ADDRESS_BAR_H: f32 = 44.0;
pub const STATUS_BAR_H: f32 = 26.0;
pub const NAV_MIN: f32 = 180.0;
pub const NAV_MAX: f32 = 420.0;

// ---------------------------------------------------------------------------
// View modes
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    ExtraLarge,
    Large,
    Medium,
    Small,
    List,
    Details,
}

impl ViewMode {
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::ExtraLarge => t("Очень крупные значки"),
            ViewMode::Large => t("Крупные значки"),
            ViewMode::Medium => t("Средние значки"),
            ViewMode::Small => t("Мелкие значки"),
            ViewMode::List => t("Список"),
            ViewMode::Details => t("Таблица"),
        }
    }

    /// Logical icon box size and cell size for the grid views.
    pub fn cell(self) -> (f32, f32) {
        // (icon, cell width)
        match self {
            ViewMode::ExtraLarge => (128.0, 196.0),
            ViewMode::Large => (96.0, 152.0),
            ViewMode::Medium => (64.0, 120.0),
            ViewMode::Small => (40.0, 100.0),
            ViewMode::List => (16.0, 0.0),
            ViewMode::Details => (20.0, 0.0),
        }
    }

    pub fn cell_height(self) -> f32 {
        match self {
            ViewMode::ExtraLarge => 176.0,
            ViewMode::Large => 136.0,
            ViewMode::Medium => 104.0,
            ViewMode::Small => 84.0,
            ViewMode::List => 24.0,
            ViewMode::Details => 27.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

pub struct RenameState {
    pub path: PathBuf,
    pub text: String,
    pub focus: bool,
}

/// State of a rubber-band (marquee) selection started on empty space.
pub struct BandSelect {
    pub start: Pos2,
    /// Selection that existed before the band started (Ctrl+drag).
    pub base: HashSet<PathBuf>,
    pub additive: bool,
    /// Becomes true once the pointer has moved enough to be a real drag.
    pub active: bool,
}

pub struct Tab {
    pub id: u64,
    pub location: Location,
    pub history: Vec<Location>,
    pub history_pos: usize,
    pub entries: Vec<Entry>,
    pub selection: Selection,
    pub sort: SortKey,
    pub descending: bool,
    pub view: ViewMode,
    pub address_edit: Option<String>,
    pub address_focus: bool,
    pub search: String,
    pub search_results: Vec<Entry>,
    pub searching: bool,
    pub search_generation: u64,
    /// Set while the network tree walk in this tab is still running.
    pub networking: bool,
    pub net_generation: u64,
    pub error: Option<String>,
    pub renaming: Option<RenameState>,
    pub scroll_to: Option<PathBuf>,
    pub show_hidden: bool,
    pub last_clicked: Option<(PathBuf, Instant)>,
    pub band: Option<BandSelect>,
}

impl Tab {
    pub fn new(id: u64, location: Location) -> Self {
        let mut tab = Self {
            id,
            location: Location::Home,
            history: vec![Location::Home],
            history_pos: 0,
            entries: Vec::new(),
            selection: Selection::default(),
            sort: SortKey::Name,
            descending: false,
            view: ViewMode::Details,
            address_edit: None,
            address_focus: false,
            search: String::new(),
            search_results: Vec::new(),
            searching: false,
            search_generation: 0,
            networking: false,
            net_generation: 0,
            error: None,
            renaming: None,
            scroll_to: None,
            show_hidden: false,
            last_clicked: None,
            band: None,
        };
        tab.set_location(location);
        tab
    }

    pub fn reload(&mut self) {
        self.error = None;
        match self.location.dir() {
            Some(path) => match fs::read_dir(path) {
                Ok(mut entries) => {
                    if !self.show_hidden {
                        entries.retain(|e| !e.hidden);
                    }
                    fs::sort_entries(&mut entries, self.sort, self.descending);
                    self.entries = entries;
                }
                Err(err) => {
                    self.entries.clear();
                    self.error = Some(crate::app::error_text(&err));
                }
            },
            None => {
                // The network view keeps its (still streaming) list; Home and
                // Gallery render from the app itself.
                if !matches!(self.location, Location::Network) {
                    self.entries.clear();
                }
            }
        }
        // Drop selection entries that no longer exist.
        let valid: HashSet<PathBuf> = self.entries.iter().map(|e| e.path.clone()).collect();
        self.selection.items.retain(|p| valid.contains(p));
    }

    pub fn set_location(&mut self, location: Location) {
        self.location = location;
        self.selection.clear();
        self.renaming = None;
        self.band = None;
        self.address_edit = None;
        self.address_focus = false;
        self.search.clear();
        self.search_results.clear();
        self.searching = false;
        self.search_generation = 0;
        self.error = None;
        self.scroll_to = None;
        self.reload();
    }

    pub fn go(&mut self, location: Location) {
        if self.location == location {
            self.reload();
            return;
        }
        self.history.truncate(self.history_pos + 1);
        self.history.push(location.clone());
        self.history_pos = self.history.len() - 1;
        self.set_location(location);
    }

    pub fn can_back(&self) -> bool {
        self.history_pos > 0
    }

    pub fn can_forward(&self) -> bool {
        self.history_pos + 1 < self.history.len()
    }

    pub fn back(&mut self) {
        if self.can_back() {
            self.history_pos -= 1;
            let loc = self.history[self.history_pos].clone();
            self.set_location(loc);
        }
    }

    pub fn forward(&mut self) {
        if self.can_forward() {
            self.history_pos += 1;
            let loc = self.history[self.history_pos].clone();
            self.set_location(loc);
        }
    }

    pub fn up(&mut self) {
        let target = match &self.location {
            Location::Dir(path) => {
                // \\server\share → \\server → Сеть.
                if fs::is_unc_host(path) {
                    Some(Location::Network)
                } else {
                    path.parent().map(|p| Location::Dir(p.to_path_buf()))
                }
            }
            Location::Network => Some(Location::Home),
            _ => None,
        };
        match target {
            Some(loc) => self.go(loc),
            None => {
                if !matches!(self.location, Location::Home) {
                    self.go(Location::Home)
                }
            }
        }
    }

    pub fn title(&self) -> String {
        if !self.search.trim().is_empty() {
            return f(t("Поиск: {}"), &[&self.search.trim()]);
        }
        self.location.title()
    }

    pub fn is_searching(&self) -> bool {
        !self.search.trim().is_empty()
    }

    pub fn visible_entries(&self) -> &[Entry] {
        if self.is_searching() {
            &self.search_results
        } else {
            &self.entries
        }
    }

    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.selection.items.iter().cloned().collect()
    }

    pub fn selected_entries(&self) -> Vec<Entry> {
        self.visible_entries()
            .iter()
            .filter(|e| self.selection.contains(&e.path))
            .cloned()
            .collect()
    }

    pub fn sort_by(&mut self, key: SortKey) {
        if self.sort == key {
            self.descending = !self.descending;
        } else {
            self.sort = key;
            self.descending = false;
        }
        self.resort();
    }

    pub fn resort(&mut self) {
        fs::sort_entries(&mut self.entries, self.sort, self.descending);
        fs::sort_entries(&mut self.search_results, self.sort, self.descending);
    }

    pub fn select_all(&mut self) {
        let items: Vec<PathBuf> = self.visible_entries().iter().map(|e| e.path.clone()).collect();
        self.selection.anchor = items.first().cloned();
        self.selection.items = items.into_iter().collect();
    }
}

// ---------------------------------------------------------------------------
// Shell icon cache
// ---------------------------------------------------------------------------

pub struct IconStore {
    tx: Sender<IconRequest>,
    rx: Receiver<IconResponse>,
    cache: HashMap<u64, Option<(egui::TextureHandle, Vec2)>>,
    order: VecDeque<u64>,
    pending: HashSet<u64>,
    failed: HashSet<u64>,
}

/// Keep GPU texture memory bounded even after browsing hundreds of folders.
const MAX_ICON_CACHE: usize = 900;
/// Request at most this many shell images at once; visible rows are served
/// first and the rest are queued on subsequent frames.
const MAX_ICON_PENDING: usize = 64;
const MAX_ICON_FAILURES: usize = 4000;

fn icon_key(path: &Path, size: u32) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    size.hash(&mut hasher);
    hasher.finish()
}

fn shell_bucket(size: f32) -> u32 {
    match size {
        s if s <= 24.0 => 32,
        s if s <= 40.0 => 48,
        s if s <= 64.0 => 64,
        s if s <= 96.0 => 96,
        s if s <= 128.0 => 128,
        _ => 256,
    }
}

impl IconStore {
    pub fn new() -> Self {
        let (tx, rx) = shell::spawn_icon_worker();
        Self {
            tx,
            rx,
            cache: HashMap::new(),
            order: VecDeque::new(),
            pending: HashSet::new(),
            failed: HashSet::new(),
        }
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        let mut responses = Vec::new();
        while let Ok(response) = self.rx.try_recv() {
            responses.push(response);
        }
        let received = !responses.is_empty();
        for response in responses {
            self.pending.remove(&response.key);
            match response.image {
                Some(image) => {
                    let texture = ctx.load_texture(
                        format!("shell-icon-{}", response.key),
                        image,
                        egui::TextureOptions::LINEAR,
                    );
                    let size = texture.size_vec2();
                    self.cache.insert(response.key, Some((texture, size)));
                    self.order.push_back(response.key);
                }
                None => {
                    self.failed.insert(response.key);
                    self.cache.insert(response.key, None);
                    self.order.push_back(response.key);
                }
            }
        }
        while self.cache.len() > MAX_ICON_CACHE {
            let Some(oldest) = self.order.pop_front() else {
                // All ordering hints were consumed (e.g. after repeated cache
                // hits); drop everything that is not currently in flight.
                self.cache.retain(|key, _| self.pending.contains(key));
                break;
            };
            if !self.pending.contains(&oldest) {
                self.cache.remove(&oldest);
            }
        }
        if self.failed.len() > MAX_ICON_FAILURES {
            self.failed.clear();
        }
        if received || !self.pending.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(60));
        }
    }

    pub fn texture(
        &mut self,
        path: &Path,
        display_size: f32,
    ) -> Option<(egui::TextureId, Vec2)> {
        let size = shell_bucket(display_size);
        let key = icon_key(path, size);
        if let Some(slot) = self.cache.get(&key) {
            return slot.as_ref().map(|(texture, dims)| (texture.id(), *dims));
        }
        if !self.pending.contains(&key) && !self.failed.contains(&key) && self.pending.len() < MAX_ICON_PENDING {
            if self.tx.send(IconRequest {
                key,
                path: path.to_path_buf(),
                size,
            }).is_ok() {
                self.pending.insert(key);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Menus, clipboard, actions
// ---------------------------------------------------------------------------

pub struct Toast {
    pub text: String,
    pub born: f64,
}

#[derive(Clone)]
pub enum MenuTarget {
    Create,
    Sort,
    View,
    More,
    Item(Vec<PathBuf>, bool),
    Background(PathBuf),
}

pub struct MenuState {
    pub target: MenuTarget,
    pub pos: Pos2,
    pub opened_at: f64,
}

#[derive(Clone)]
pub struct Clipboard {
    pub paths: Vec<PathBuf>,
    pub cut: bool,
}

pub enum Action {
    Navigate(Location),
    Back,
    Forward,
    Up,
    Refresh,
    Open(Vec<PathBuf>),
    OpenNewWindow(PathBuf),
    OpenWith(PathBuf),
    OpenTerminal(PathBuf),
    Cut,
    Copy,
    Paste,
    CopyPath(Vec<PathBuf>),
    DeleteRequest(Vec<PathBuf>, bool),
    RenameStart(PathBuf),
    RenameCommit,
    RenameCancel,
    Properties(PathBuf),
    NewFolder,
    NewTextFile,
    SortBy(SortKey),
    SetView(ViewMode),
    ToggleHidden,
    ToggleTheme,
    SetTheme(ThemeMode),
    SetAccent(Accent),
    SetCustomAccent(Color32),
    SetShowHidden(bool),
    ToggleEditMode,
    HideQuick(PathBuf),
    RestoreQuick(PathBuf),
    HideNav(String, String),
    RestoreNav(String, String),
    ResetHiddenItems,
    OpenSettings,
    FocusSearch,
    NewTab,
    CloseTab(usize),
    SelectTab(usize),
    SelectAll,
    OpenMenu(MenuTarget, Pos2),
    CloseMenu,
    ConfirmDelete,
    DoDelete(Vec<PathBuf>, bool),
    CancelDialog,
    ApplyAttributes(PathBuf, bool, bool),
    Toast(String),
    SetLang(Lang),
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

pub enum SpecialMsg {
    Recent(Vec<Entry>),
    Gallery(Vec<Entry>),
}

pub struct ExplorerApp {
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub next_tab_id: u64,
    pub mode: ThemeMode,
    pub accent: Accent,
    /// User-picked accent colour, used with `Accent::Custom`.
    pub custom_accent: Color32,
    pub system_dark: bool,
    pub palette: Palette,
    applied_mode: ThemeMode,
    applied_accent: Accent,
    applied_custom: Color32,
    last_system_check: f64,
    pub ctx: egui::Context,
    pub drives: Vec<Drive>,
    pub quick: Vec<QuickFolder>,
    /// Quick-access folders removed by the user in edit mode.
    pub hidden_quick: HashSet<PathBuf>,
    /// Other navigation-pane items removed in edit mode (`home`, `gallery`,
    /// `network`, `drive:<mount>`).
    pub hidden_nav: HashSet<String>,
    /// When true, nav/Home items show an × button that hides them.
    pub edit_mode: bool,
    pub clipboard: Option<Clipboard>,
    pub dialog: Dialog,
    pub toasts: Vec<Toast>,
    pub icons: IconStore,
    pub menu: Option<MenuState>,
    pub search_tx: Sender<SearchMsg>,
    pub search_rx: Receiver<SearchMsg>,
    pub search_seq: u64,
    pub search_stop: Arc<AtomicU64>,
    pub special_tx: Sender<SpecialMsg>,
    pub special_rx: Receiver<SpecialMsg>,
    /// Network ("Сеть") discovery stream and its cached result.
    pub net_tx: Sender<NetworkMsg>,
    pub net_rx: Receiver<NetworkMsg>,
    pub net_seq: u64,
    pub net_stop: Arc<AtomicU64>,
    pub network_cache: Vec<Entry>,
    recent_loading: bool,
    gallery_loading: bool,
    recent_loaded: bool,
    gallery_loaded: bool,
    pub nav_width: f32,
    pub recent: Vec<Entry>,
    pub gallery: Vec<Entry>,
    pub last_title: String,
    frame_time: f64,
    /// Window size persisted from the previous run (points).
    window_size: Option<(f32, f32)>,
    window_size_dirty: bool,
    window_size_saved_at: f64,
    /// Active interface language (mirrors `lang::current()` for the menu).
    pub lang: Lang,
}

impl ExplorerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        let prefs = Preferences::load();
        lang::set_current(prefs.lang);
        let system_dark = theme::system_uses_dark();
        let mode = prefs.theme;
        let accent = prefs.accent;
        let custom_accent = prefs.custom_accent;
        let resolved = mode.resolve(system_dark);
        let p = palette_with(resolved, accent, custom_accent);
        theme::apply_style(&cc.egui_ctx, &p);

        let start = initial_location();
        let mut first = Tab::new(1, start);
        first.show_hidden = prefs.show_hidden;
        first.reload();
        let (search_tx, search_rx) = mpsc::channel();
        let (special_tx, special_rx) = mpsc::channel();
        let (net_tx, net_rx) = mpsc::channel();

        let mut app = Self {
            tabs: vec![first],
            active: 0,
            next_tab_id: 2,
            mode,
            accent,
            custom_accent,
            system_dark,
            palette: p,
            applied_mode: resolved,
            applied_accent: accent,
            applied_custom: custom_accent,
            last_system_check: 0.0,
            ctx: cc.egui_ctx.clone(),
            drives: fs::drives(),
            quick: fs::quick_folders(),
            hidden_quick: prefs.hidden_quick.iter().cloned().collect(),
            hidden_nav: prefs.hidden_nav.iter().cloned().collect(),
            edit_mode: false,
            clipboard: None,
            dialog: Dialog::None,
            toasts: Vec::new(),
            icons: IconStore::new(),
            menu: None,
            search_tx,
            search_rx,
            search_seq: 1,
            search_stop: Arc::new(AtomicU64::new(0)),
            special_tx,
            special_rx,
            net_tx,
            net_rx,
            net_seq: 1,
            net_stop: Arc::new(AtomicU64::new(0)),
            network_cache: Vec::new(),
            recent_loading: false,
            gallery_loading: false,
            recent_loaded: false,
            gallery_loaded: false,
            nav_width: 232.0,
            recent: Vec::new(),
            gallery: Vec::new(),
            last_title: String::new(),
            frame_time: 0.0,
            window_size: prefs.window_size,
            window_size_dirty: false,
            window_size_saved_at: 0.0,
            lang: prefs.lang,
        };
        app.refresh_special(false);
        app
    }

    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active.min(self.tabs.len() - 1)]
    }

    pub fn active_tab_mut(&mut self) -> &mut Tab {
        let index = self.active.min(self.tabs.len() - 1);
        &mut self.tabs[index]
    }

    pub fn toast(&mut self, text: impl Into<String>) {
        self.toasts.push(Toast {
            text: text.into(),
            born: self.frame_time,
        });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    pub fn current_dir(&self) -> Option<PathBuf> {
        match &self.active_tab().location {
            Location::Dir(p) => Some(p.clone()),
            _ => None,
        }
    }

    pub fn paste_dir(&self) -> Option<PathBuf> {
        self.current_dir().or_else(dirs::home_dir)
    }

    pub fn toggle_theme(&mut self) {
        let current = self.mode.resolve(self.system_dark);
        self.mode = if current.is_dark() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };
        self.apply_theme();
        self.save_prefs();
    }

    pub fn apply_theme(&mut self) {
        self.system_dark = theme::system_uses_dark();
        let resolved = self.mode.resolve(self.system_dark);
        self.palette = palette_with(resolved, self.accent, self.custom_accent);
        theme::apply_style(&self.ctx, &self.palette);
        self.applied_mode = resolved;
        self.applied_accent = self.accent;
        self.applied_custom = self.custom_accent;
        self.ctx.request_repaint();
    }

    pub fn save_prefs(&self) {
        let mut hidden_quick: Vec<PathBuf> = self.hidden_quick.iter().cloned().collect();
        hidden_quick.sort();
        let mut hidden_nav: Vec<String> = self.hidden_nav.iter().cloned().collect();
        hidden_nav.sort();
        Preferences {
            theme: self.mode,
            accent: self.accent,
            custom_accent: self.custom_accent,
            show_hidden: self.active_tab().show_hidden,
            hidden_quick,
            hidden_nav,
            window_size: self.window_size,
            lang: self.lang,
        }
        .save();
    }

    /// Watches the window size, remember the user's manual resize and writes
    /// it out shortly after the resizing stops.
    fn track_window_size(&mut self, ctx: &egui::Context) {
        let (inner, maximized) = ctx.input(|i| {
            (
                i.viewport()
                    .inner_rect
                    .map(|r| (r.width(), r.height())),
                i.viewport().maximized.unwrap_or(false),
            )
        });
        if maximized {
            return;
        }
        let Some((w, h)) = inner else {
            return;
        };
        // Only meaningful sizes; ignore degenerate/placeholder values.
        let rounded = (w.round(), h.round());
        if rounded.0 < 100.0 || rounded.1 < 100.0 {
            return;
        }
        if self.window_size != Some(rounded) {
            self.window_size = Some(rounded);
            self.window_size_dirty = true;
            self.window_size_saved_at = self.frame_time;
        }
        if self.window_size_dirty && self.frame_time - self.window_size_saved_at > 1.0 {
            self.window_size_dirty = false;
            self.save_prefs();
        }
    }

    /// True when the quick-access folder was hidden in edit mode.
    pub fn quick_hidden(&self, path: &Path) -> bool {
        self.hidden_quick
            .iter()
            .any(|hidden| fs::same_path(hidden, path))
    }

    /// True when a non-quick navigation item was hidden in edit mode.
    pub fn nav_hidden(&self, key: &str) -> bool {
        self.hidden_nav.contains(key)
    }

    /// Number of navigation items currently hidden.
    pub fn hidden_item_count(&self) -> usize {
        self.hidden_quick.len() + self.hidden_nav.len()
    }

    /// Toggles the interface edit mode used to remove nav/Home elements.
    pub fn toggle_edit_mode(&mut self) {
        self.edit_mode = !self.edit_mode;
        self.menu = None;
    }

    /// Starts background scans for the Home/Gallery pages. Doing this on the
    /// UI thread would freeze the window on large user folders.
    pub fn refresh_special(&mut self, force: bool) {
        let location = self.active_tab().location.clone();
        match location {
            Location::Home => {
                self.drives = fs::drives();
                if force || !self.recent_loaded || self.recent.is_empty() {
                    self.start_recent();
                }
            }
            Location::Gallery => {
                if force || !self.gallery_loaded || self.gallery.is_empty() {
                    self.start_gallery();
                }
            }
            Location::Network => {
                self.start_network(force);
            }
            Location::Dir(_) => {}
        }
    }

    fn start_recent(&mut self) {
        if self.recent_loading {
            return;
        }
        self.recent_loading = true;
        self.recent_loaded = true;
        let tx = self.special_tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let entries = collect_recent();
            let _ = tx.send(SpecialMsg::Recent(entries));
            ctx.request_repaint();
        });
    }

    fn start_gallery(&mut self) {
        if self.gallery_loading {
            return;
        }
        self.gallery_loading = true;
        self.gallery_loaded = true;
        let tx = self.special_tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let entries = collect_gallery();
            let _ = tx.send(SpecialMsg::Gallery(entries));
            ctx.request_repaint();
        });
    }

    /// Shows the cached server list or starts a fresh network walk.
    fn start_network(&mut self, force: bool) {
        let index = self.active;
        if !matches!(self.tabs[index].location, Location::Network) {
            return;
        }
        if !force && !self.network_cache.is_empty() {
            // Instant view from the cache; break the previous walk, if any.
            self.net_seq += 1;
            let generation = self.net_seq;
            self.net_stop.store(generation, Ordering::Relaxed);
            let tab = &mut self.tabs[index];
            tab.networking = false;
            tab.net_generation = generation;
            tab.entries = self.network_cache.clone();
            return;
        }
        self.net_seq += 1;
        let generation = self.net_seq;
        self.net_stop.store(generation, Ordering::Relaxed);
        let Some(tab) = self.tabs.get_mut(index) else { return };
        tab.entries.clear();
        tab.networking = true;
        tab.net_generation = generation;
        let stop = self.net_stop.clone();
        let tx = self.net_tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            fs::discover_network(generation, stop, tx);
            ctx.request_repaint();
        });
    }

    fn poll_special(&mut self) {
        while let Ok(msg) = self.special_rx.try_recv() {
            match msg {
                SpecialMsg::Recent(entries) => {
                    self.recent_loading = false;
                    self.recent = entries;
                }
                SpecialMsg::Gallery(entries) => {
                    self.gallery_loading = false;
                    self.gallery = entries;
                }
            }
        }
        // Progressive network walk: merge batches into every tab that still
        // shows this view, then keep the cache for instant reopening.
        while let Ok(msg) = self.net_rx.try_recv() {
            match msg {
                NetworkMsg::Batch { generation, entries } => {
                    for tab in &mut self.tabs {
                        if matches!(tab.location, Location::Network)
                            && tab.net_generation == generation
                        {
                            let key = tab.sort;
                            let descending = tab.descending;
                            fs::merge_sorted(&mut tab.entries, entries.clone(), key, descending);
                        }
                    }
                    fs::merge_sorted(
                        &mut self.network_cache,
                        entries,
                        SortKey::Name,
                        false,
                    );
                }
                NetworkMsg::Done { generation } => {
                    for tab in &mut self.tabs {
                        if tab.net_generation == generation {
                            tab.networking = false;
                            if matches!(tab.location, Location::Network) {
                                self.network_cache = tab.entries.clone();
                            }
                        }
                    }
                }
            }
        }
        let loading = self
            .tabs
            .iter()
            .any(|t| t.networking)
            || self.recent_loading
            || self.gallery_loading;
        if loading {
            self.ctx
                .request_repaint_after(Duration::from_millis(150));
        }
    }

    pub fn start_search(&mut self, ctx: &egui::Context) {
        let query = self.active_tab().search.trim().to_string();
        let index = self.active;
        self.search_seq += 1;
        let generation = self.search_seq;
        let Some(tab) = self.tabs.get_mut(index) else { return };
        tab.search_generation = generation;
        tab.search_results.clear();
        if query.is_empty() {
            tab.searching = false;
            return;
        }
        tab.searching = true;
        let root = tab
            .location
            .dir()
            .map(|p| p.to_path_buf())
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("C:\\"));
        self.search_stop.store(generation, Ordering::Relaxed);
        let stop = self.search_stop.clone();
        let tx = self.search_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            fs::search_recursive(root, query, generation, stop, tx);
            ctx.request_repaint();
        });
    }

    fn poll_search(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.search_rx.try_recv() {
            match msg {
                SearchMsg::Batch { generation, entries } => {
                    if let Some(tab) = self.tabs.get_mut(self.active) {
                        if tab.search_generation == generation && tab.is_searching() {
                            let key = tab.sort;
                            let descending = tab.descending;
                            fs::merge_sorted(
                                &mut tab.search_results,
                                entries,
                                key,
                                descending,
                            );
                            if tab.search_results.len() > 4000 {
                                tab.search_results.truncate(4000);
                            }
                        }
                    }
                }
                SearchMsg::Done { generation, total } => {
                    if let Some(tab) = self.tabs.get_mut(self.active) {
                        if tab.search_generation == generation {
                            tab.searching = false;
                        }
                    }
                    let _ = total;
                }
            }
        }
        let searching = self.tabs.iter().any(|t| t.searching);
        if searching {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
    }

    fn navigate_active(&mut self, location: Location) {
        let index = self.active;
        self.tabs[index].go(location);
        self.refresh_special(false);
    }

    pub fn apply(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::Navigate(location) => {
                self.navigate_active(location);
            }
            Action::Back => {
                self.active_tab_mut().back();
                self.refresh_special(false);
            }
            Action::Forward => {
                self.active_tab_mut().forward();
                self.refresh_special(false);
            }
            Action::Up => {
                self.active_tab_mut().up();
                self.refresh_special(false);
            }
            Action::Refresh => {
                self.active_tab_mut().reload();
                self.drives = fs::drives();
                self.quick = fs::quick_folders();
                self.refresh_special(true);
            }
            Action::Open(paths) => {
                let mut navigated = false;
                for path in &paths {
                    if path.is_dir() || fs::is_unc_host(path) {
                        if paths.len() == 1 {
                            self.navigate_active(Location::Dir(path.clone()));
                            navigated = true;
                        }
                    } else if !shell::open_path(path) {
                        self.dialog = Dialog::Error {
                            title: t("Не удается открыть файл").into(),
                            message: f(
                                t("Не удалось открыть «{}». Проверьте, что файл существует и связано приложение."),
                                &[&path.display().to_string()],
                            ),
                        };
                    }
                }
                if !navigated {
                    // Opening files keeps the folder in place.
                }
            }
            Action::OpenNewWindow(path) => {
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).arg(&path).spawn();
                }
            }
            Action::OpenWith(path) => {
                shell::open_with(&path);
            }
            Action::OpenTerminal(path) => {
                shell::open_terminal(&path);
            }
            Action::Cut => {
                let paths = self.active_tab().selected_paths();
                if !paths.is_empty() {
                    self.clipboard = Some(Clipboard { paths, cut: true });
                    self.toast(t("Элементы вырезаны"));
                }
            }
            Action::Copy => {
                let paths = self.active_tab().selected_paths();
                if !paths.is_empty() {
                    self.clipboard = Some(Clipboard { paths, cut: false });
                    self.toast(t("Элементы скопированы"));
                }
            }
            Action::CopyPath(paths) => {
                let text = paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\r\n");
                ctx.copy_text(text);
                self.toast(t("Путь скопирован"));
            }
            Action::Paste => {
                let Some(clip) = self.clipboard.clone() else { return };
                let Some(target) = self.paste_dir() else { return };
                match fs::copy_into(&target, &clip.paths, clip.cut) {
                    Ok(count) => {
                        if clip.cut {
                            self.clipboard = None;
                        }
                        self.active_tab_mut().reload();
                        self.toast(f(t("Вставлено: {}"), &[&count.to_string()]));
                    }
                    Err(err) => {
                        self.dialog = Dialog::Error {
                            title: t("Не удается вставить элементы").into(),
                            message: err,
                        };
                    }
                }
            }
            Action::DeleteRequest(paths, permanent) => {
                if paths.is_empty() {
                    return;
                }
                let single = paths.len() == 1;
                let label = if single {
                    paths[0]
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| paths[0].display().to_string())
                } else {
                    f(t("{} элементов"), &[&paths.len().to_string()])
                };
                let is_dir = paths[0].is_dir();
                let _ = ctx;
                self.dialog = Dialog::Delete {
                    paths,
                    permanent,
                    label,
                    is_dir,
                    single,
                };
            }
            Action::DoDelete(paths, permanent) => {
                match fs::delete_items(&paths, permanent) {
                    Ok(count) => {
                        self.active_tab_mut().selection.clear();
                        self.active_tab_mut().reload();
                        if permanent {
                            self.toast(f(t("Удалено безвозвратно: {}"), &[&count.to_string()]));
                        } else {
                            self.toast(f(t("Перемещено в корзину: {}"), &[&count.to_string()]));
                        }
                    }
                    Err(err) => {
                        self.dialog = Dialog::Error {
                            title: t("Не удается удалить элемент").into(),
                            message: err,
                        };
                    }
                }
            }
            Action::ConfirmDelete => {
                if let Dialog::Delete { paths, permanent, .. } = std::mem::replace(&mut self.dialog, Dialog::None) {
                    self.apply(Action::DoDelete(paths, permanent), ctx);
                }
            }
            Action::CancelDialog => {
                self.dialog = Dialog::None;
            }
            Action::RenameStart(path) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let tab = self.active_tab_mut();
                tab.selection.select_only(&path);
                tab.renaming = Some(RenameState {
                    path: path.clone(),
                    text: name,
                    focus: true,
                });
            }
            Action::RenameCommit => {
                let Some(state) = self.active_tab_mut().renaming.take() else { return };
                let name = state.text.trim().to_string();
                let old_name = state
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if name.is_empty() || name == old_name {
                    return;
                }
                match fs::rename(&state.path, &name) {
                    Ok(new_path) => {
                        self.active_tab_mut().reload();
                        self.active_tab_mut().selection.select_only(&new_path);
                        self.active_tab_mut().scroll_to = Some(new_path);
                    }
                    Err(err) => {
                        self.dialog = Dialog::Error {
                            title: t("Не удается переименовать").into(),
                            message: format!("{}\n\n{}", error_text(&err), state.path.display()),
                        };
                    }
                }
            }
            Action::RenameCancel => {
                self.active_tab_mut().renaming = None;
            }
            Action::Properties(path) => {
                self.dialog = Dialog::Properties(PropertiesState::new(path));
            }
            Action::ApplyAttributes(path, read_only, hidden) => {
                match fs::set_attributes(&path, read_only, hidden) {
                    Ok(_) => {
                        self.active_tab_mut().reload();
                        self.dialog = Dialog::None;
                        self.toast(t("Атрибуты изменены"));
                    }
                    Err(err) => {
                        self.dialog = Dialog::Error {
                            title: t("Не удается изменить атрибуты").into(),
                            message: error_text(&err),
                        };
                    }
                }
            }
            Action::NewFolder => {
                let Some(dir) = self.paste_dir() else { return };
                match fs::create_folder(&dir) {
                    Ok(path) => {
                        self.active_tab_mut().reload();
                        self.active_tab_mut().selection.select_only(&path);
                        self.active_tab_mut().scroll_to = Some(path.clone());
                        self.active_tab_mut().renaming = Some(RenameState {
                            path,
                            text: t("Новая папка").into(),
                            focus: true,
                        });
                    }
                    Err(err) => {
                        self.dialog = Dialog::Error {
                            title: t("Не удается создать папку").into(),
                            message: f(
                                t("{}\n\nУ вас может не быть разрешения на создание папки в этом расположении."),
                                &[&error_text(&err)],
                            ),
                        };
                    }
                }
            }
            Action::NewTextFile => {
                let Some(dir) = self.paste_dir() else { return };
                match fs::create_text_file(&dir) {
                    Ok(path) => {
                        self.active_tab_mut().reload();
                        self.active_tab_mut().selection.select_only(&path);
                        self.active_tab_mut().scroll_to = Some(path.clone());
                        self.active_tab_mut().renaming = Some(RenameState {
                            path,
                            text: t("Новый текстовый документ").into(),
                            focus: true,
                        });
                    }
                    Err(err) => {
                        self.dialog = Dialog::Error {
                            title: t("Не удается создать файл").into(),
                            message: error_text(&err),
                        };
                    }
                }
            }
            Action::SortBy(key) => {
                self.active_tab_mut().sort_by(key);
            }
            Action::SetView(view) => {
                self.active_tab_mut().view = view;
            }
            Action::ToggleHidden => {
                let show = !self.active_tab().show_hidden;
                for tab in &mut self.tabs {
                    tab.show_hidden = show;
                    tab.reload();
                }
            }
            Action::ToggleTheme => {
                self.toggle_theme();
            }
            Action::SetTheme(mode) => {
                if self.mode != mode {
                    self.mode = mode;
                    self.apply_theme();
                    self.save_prefs();
                }
            }
            Action::SetAccent(accent) => {
                if self.accent != accent {
                    self.accent = accent;
                    self.apply_theme();
                    self.save_prefs();
                }
            }
            Action::SetCustomAccent(color) => {
                self.custom_accent = color;
                self.accent = Accent::Custom;
                self.apply_theme();
                // Colour pickers emit a change every frame while dragging;
                // persist once the pointer is released.
                if !self.ctx.input(|i| i.pointer.any_down()) {
                    self.save_prefs();
                }
            }
            Action::SetShowHidden(show) => {
                for tab in &mut self.tabs {
                    if tab.show_hidden != show {
                        tab.show_hidden = show;
                        tab.reload();
                    }
                }
                self.save_prefs();
            }
            Action::OpenSettings => {
                self.dialog = Dialog::Settings;
                self.menu = None;
            }
            Action::ToggleEditMode => {
                self.toggle_edit_mode();
                if self.edit_mode {
                    self.toast(t("Режим редактирования: × убирает элемент, + возвращает его"));
                } else {
                    self.toast(t("Режим редактирования выключен"));
                }
            }
            Action::HideQuick(path) => {
                let title = self
                    .quick
                    .iter()
                    .find(|folder| fs::same_path(&folder.path, &path))
                    .map(|folder| folder.title)
                    .unwrap_or(t("Элемент"));
                self.hidden_quick.insert(path);
                self.save_prefs();
                self.toast(f(
                    t("«{title}» убран — вернуть можно кнопкой + в режиме редактирования"),
                    &[&title],
                ));
            }
            Action::RestoreQuick(path) => {
                let title = self
                    .quick
                    .iter()
                    .find(|folder| fs::same_path(&folder.path, &path))
                    .map(|folder| folder.title)
                    .unwrap_or(t("Элемент"));
                self.hidden_quick
                    .retain(|hidden| !fs::same_path(hidden, &path));
                self.save_prefs();
                self.toast(f(t("«{title}» снова на панели навигации"), &[&title]));
            }
            Action::HideNav(key, title) => {
                self.hidden_nav.insert(key);
                self.save_prefs();
                self.toast(f(
                    t("«{title}» убран — вернуть можно кнопкой + в режиме редактирования"),
                    &[&title],
                ));
            }
            Action::RestoreNav(key, title) => {
                self.hidden_nav.remove(&key);
                self.save_prefs();
                self.toast(f(t("«{title}» снова на панели навигации"), &[&title]));
            }
            Action::ResetHiddenItems => {
                if self.hidden_item_count() > 0 {
                    self.hidden_quick.clear();
                    self.hidden_nav.clear();
                    self.save_prefs();
                    self.toast(t("Скрытые элементы восстановлены"));
                }
            }
            Action::FocusSearch => {
                ctx.memory_mut(|m| m.request_focus(Id::new("search-edit")));
            }
            Action::NewTab => {
                let location = self.active_tab().location.clone();
                let id = self.next_tab_id;
                self.next_tab_id += 1;
                let mut tab = Tab::new(id, location);
                tab.view = self.active_tab().view;
                tab.sort = self.active_tab().sort;
                tab.descending = self.active_tab().descending;
                tab.show_hidden = self.active_tab().show_hidden;
                self.tabs.push(tab);
                self.active = self.tabs.len() - 1;
                self.refresh_special(false);
            }
            Action::CloseTab(index) => {
                if self.tabs.len() <= 1 {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                    return;
                }
                if index < self.tabs.len() {
                    self.tabs.remove(index);
                    if self.active >= self.tabs.len() {
                        self.active = self.tabs.len() - 1;
                    } else if index < self.active {
                        self.active -= 1;
                    }
                    self.refresh_special(false);
                }
            }
            Action::SelectTab(index) => {
                if index < self.tabs.len() {
                    self.active = index;
                    self.menu = None;
                }
            }
            Action::SelectAll => {
                self.active_tab_mut().select_all();
            }
            Action::OpenMenu(target, pos) => {
                self.menu = Some(MenuState {
                    target,
                    pos,
                    opened_at: self.frame_time,
                });
            }
            Action::CloseMenu => {
                self.menu = None;
            }
            Action::Toast(text) => self.toast(text),
            Action::SetLang(lang) => {
                self.lang = lang;
                lang::set_current(lang);
                self.save_prefs();
            }
        }
        // Menus, dialogs and toasts change outside of input events; keep frames
        // coming so their appearance/disappearance is rendered promptly.
        ctx.request_repaint();
    }

    // -----------------------------------------------------------------------
    // Menu rendering
    // -----------------------------------------------------------------------

    fn show_menu(&mut self, ctx: &egui::Context) {
        let Some(menu) = self.menu.take() else { return };
        let p = self.palette;
        let mut actions: Vec<Action> = Vec::new();
        let time = ctx.input(|i| i.time);
        let screen = ctx.content_rect();
        let target = menu.target.clone();

        // Rough height estimate so the popup can flip above the anchor.
        let estimated = match &target {
            MenuTarget::Item(..) => 340.0,
            MenuTarget::Background(_) => 330.0,
            MenuTarget::Create => 130.0,
            MenuTarget::Sort => 230.0,
            MenuTarget::View => 250.0,
            MenuTarget::More => 70.0,
        };
        let mut pos = menu.pos;
        // Keep the popup as close to the cursor as possible: only shift it when
        // it would otherwise leave the window.
        let margin = 8.0;
        let min_x = screen.left() + margin;
        let max_x = (screen.right() - widgets::MENU_WIDTH - margin).max(min_x);
        let min_y = screen.top() + margin;
        let max_y = (screen.bottom() - estimated - margin).max(min_y);
        pos.x = pos.x.clamp(min_x, max_x);
        pos.y = pos.y.clamp(min_y, max_y);

        let menu_id = Id::new("explorer-popup-menu");
        let area = egui::Area::new(menu_id)
            .order(Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(p.menu)
                    .stroke(Stroke::new(1.0, p.border_strong))
                    .inner_margin(egui::Margin::symmetric(4, 5))
                    .show(ui, |ui| {
                        ui.set_width(widgets::MENU_WIDTH - 8.0);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        let selected = self.active_tab().selected_paths();
                        let tab = self.active_tab();
                        match &target {
                            MenuTarget::Create => {
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::NEW_FOLDER,
                                    t("Папка"),
                                    "Ctrl+Shift+N",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::NewFolder);
                                }
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::DOCUMENT,
                                    t("Текстовый документ"),
                                    "",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::NewTextFile);
                                }
                                widgets::menu_separator(ui, &p);
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::LINK,
                                    t("Ярлык"),
                                    "",
                                    false,
                                )
                                .clicked()
                                {
                                    actions.push(Action::Toast(
                                        t("Создание ярлыков будет добавлено позже").into(),
                                    ));
                                }
                            }
                            MenuTarget::Sort => {
                                widgets::menu_header(ui, &p, t("Сортировать по"));
                                for key in [SortKey::Name, SortKey::Modified, SortKey::Type, SortKey::Size] {
                                    if widgets::menu_check(ui, &p, key.label(), tab.sort == key).clicked() {
                                        actions.push(Action::SortBy(key));
                                    }
                                }
                                widgets::menu_separator(ui, &p);
                                if widgets::menu_check(ui, &p, t("По возрастанию"), !tab.descending).clicked() {
                                    if tab.descending {
                                        actions.push(Action::SortBy(tab.sort));
                                    }
                                }
                                if widgets::menu_check(ui, &p, t("По убыванию"), tab.descending).clicked() {
                                    if !tab.descending {
                                        actions.push(Action::SortBy(tab.sort));
                                    }
                                }
                            }
                            MenuTarget::View => {
                                widgets::menu_header(ui, &p, t("Просмотреть как"));
                                for view in [
                                    ViewMode::ExtraLarge,
                                    ViewMode::Large,
                                    ViewMode::Medium,
                                    ViewMode::Small,
                                    ViewMode::List,
                                    ViewMode::Details,
                                ] {
                                    if widgets::menu_check(ui, &p, view.label(), tab.view == view).clicked() {
                                        actions.push(Action::SetView(view));
                                    }
                                }
                                widgets::menu_separator(ui, &p);
                                if widgets::menu_check(
                                    ui,
                                    &p,
                                    t("Показать скрытые элементы"),
                                    tab.show_hidden,
                                )
                                .clicked()
                                {
                                    actions.push(Action::ToggleHidden);
                                }
                            }
                            MenuTarget::More => {
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::SETTINGS,
                                    t("Параметры проводника"),
                                    "",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::OpenSettings);
                                }
                            }
                            MenuTarget::Item(paths, is_dir) => {
                                let single = paths.len() == 1 && *is_dir;
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::OPEN_FOLDER,
                                    t("Открыть"),
                                    "",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::Open(paths.clone()));
                                }
                                if !single {
                                    if widgets::menu_item(
                                        ui,
                                        &p,
                                        crate::icons::glyph::PAGE,
                                        t("Открыть с помощью"),
                                        "",
                                        paths.len() == 1,
                                    )
                                    .clicked()
                                    {
                                        if let Some(path) = paths.first() {
                                            actions.push(Action::OpenWith(path.clone()));
                                        }
                                    }
                                }
                                if paths.len() == 1 {
                                    if widgets::menu_item(
                                        ui,
                                        &p,
                                        crate::icons::glyph::OPEN_FOLDER,
                                        t("Открыть в новом окне"),
                                        "",
                                        *is_dir,
                                    )
                                    .clicked()
                                    {
                                        actions.push(Action::OpenNewWindow(paths[0].clone()));
                                    }
                                }
                                widgets::menu_separator(ui, &p);
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::CUT,
                                    t("Вырезать"),
                                    "Ctrl+X",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::Cut);
                                }
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::COPY,
                                    t("Копировать"),
                                    "Ctrl+C",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::Copy);
                                }
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::RENAME,
                                    t("Переименовать"),
                                    "F2",
                                    paths.len() == 1,
                                )
                                .clicked()
                                {
                                    actions.push(Action::RenameStart(paths[0].clone()));
                                }
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::DELETE,
                                    t("Удалить"),
                                    "Delete",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::DeleteRequest(paths.clone(), false));
                                }
                                widgets::menu_separator(ui, &p);
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::LINK,
                                    t("Копировать путь"),
                                    "",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::CopyPath(paths.clone()));
                                }
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::PROPERTIES,
                                    t("Свойства"),
                                    "Alt+Enter",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::Properties(paths[0].clone()));
                                }
                            }
                            MenuTarget::Background(dir) => {
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::PASTE,
                                    t("Вставить"),
                                    "Ctrl+V",
                                    self.clipboard.is_some(),
                                )
                                .clicked()
                                {
                                    actions.push(Action::Paste);
                                }
                                widgets::menu_separator(ui, &p);
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::NEW_FOLDER,
                                    t("Создать папку"),
                                    "Ctrl+Shift+N",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::NewFolder);
                                }
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::DOCUMENT,
                                    t("Создать текстовый документ"),
                                    "",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::NewTextFile);
                                }
                                widgets::menu_separator(ui, &p);
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::SETTINGS,
                                    t("Открыть в Терминале"),
                                    "",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::OpenTerminal(dir.clone()));
                                }
                                if widgets::menu_item(
                                    ui,
                                    &p,
                                    crate::icons::glyph::INFO,
                                    t("Свойства"),
                                    "",
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(Action::Properties(dir.clone()));
                                }
                                let _ = selected;
                            }
                        }
                    });
            });

        let pointer = ctx.input(|i| i.pointer.interact_pos());
        let clicked_out = ctx.input(|i| i.pointer.any_click())
            && pointer.map_or(true, |p| !area.response.rect.contains(p));
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let close = !actions.is_empty()
            || (clicked_out && time - menu.opened_at > 0.12)
            || (escape && time - menu.opened_at > 0.05);
        if !close {
            self.menu = Some(menu);
        }
        for action in actions {
            self.apply(action, ctx);
        }
    }

    // -----------------------------------------------------------------------
    // Window edges
    // -----------------------------------------------------------------------

    fn edge_resize(&mut self, ui: &Ui, ctx: &egui::Context) {
        let rect = ui.max_rect();
        let border = 6.0;
        let handles = [
            (
                Rect::from_min_size(rect.min, Vec2::new(rect.width(), border)),
                CursorIcon::ResizeNorth,
                egui::viewport::ResizeDirection::North,
            ),
            (
                Rect::from_min_size(
                    Pos2::new(rect.left(), rect.bottom() - border),
                    Vec2::new(rect.width(), border),
                ),
                CursorIcon::ResizeSouth,
                egui::viewport::ResizeDirection::South,
            ),
            (
                Rect::from_min_size(rect.min, Vec2::new(border, rect.height())),
                CursorIcon::ResizeWest,
                egui::viewport::ResizeDirection::West,
            ),
            (
                Rect::from_min_size(
                    Pos2::new(rect.right() - border, rect.top()),
                    Vec2::new(border, rect.height()),
                ),
                CursorIcon::ResizeEast,
                egui::viewport::ResizeDirection::East,
            ),
            (
                Rect::from_min_size(rect.min, Vec2::splat(border * 2.0)),
                CursorIcon::ResizeNorthWest,
                egui::viewport::ResizeDirection::NorthWest,
            ),
            (
                Rect::from_min_size(
                    Pos2::new(rect.right() - border * 2.0, rect.top()),
                    Vec2::splat(border * 2.0),
                ),
                CursorIcon::ResizeNorthEast,
                egui::viewport::ResizeDirection::NorthEast,
            ),
            (
                Rect::from_min_size(
                    Pos2::new(rect.left(), rect.bottom() - border * 2.0),
                    Vec2::splat(border * 2.0),
                ),
                CursorIcon::ResizeSouthWest,
                egui::viewport::ResizeDirection::SouthWest,
            ),
            (
                Rect::from_min_size(
                    Pos2::new(rect.right() - border * 2.0, rect.bottom() - border * 2.0),
                    Vec2::splat(border * 2.0),
                ),
                CursorIcon::ResizeSouthEast,
                egui::viewport::ResizeDirection::SouthEast,
            ),
        ];
        for (index, (r, cursor, direction)) in handles.iter().enumerate() {
            let response = ui.interact(
                *r,
                Id::new(("window-edge", index)),
                Sense::click_and_drag(),
            );
            if response.hovered() || response.dragged() {
                ctx.set_cursor_icon(*cursor);
            }
            if response.drag_started() {
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(*direction));
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) -> Vec<Action> {
        let mut actions = Vec::new();
        if !matches!(self.dialog, Dialog::None) {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                actions.push(Action::CancelDialog);
            }
            return actions;
        }

        let typing = ctx.egui_wants_keyboard_input();
        let renaming = self.active_tab().renaming.is_some();
        let address_focus = self.active_tab().address_focus;
        let editing = typing && (address_focus || renaming);

        let mods = ctx.input(|i| i.modifiers);
        let cmd = mods.ctrl || mods.command;
        let shift = mods.shift;
        let key = |k: egui::Key| ctx.input(|i| i.key_pressed(k));

        // Mouse 4/5 — назад/вперёд, как в проводнике Windows.
        let side = ctx.input(|i| {
            (
                i.pointer.button_pressed(egui::PointerButton::Extra1),
                i.pointer.button_pressed(egui::PointerButton::Extra2),
            )
        });
        if side.0 {
            actions.push(Action::Back);
        }
        if side.1 {
            actions.push(Action::Forward);
        }

        if key(egui::Key::Escape) {
            actions.push(Action::CloseMenu);
            if self.edit_mode {
                actions.push(Action::ToggleEditMode);
            } else if !editing {
                self.active_tab_mut().selection.clear();
            }
        }
        if editing {
            return actions;
        }

        if cmd && key(egui::Key::L) {
            let tab = self.active_tab_mut();
            tab.address_focus = true;
            tab.address_edit = Some(tab.location.display());
        }
        if cmd && key(egui::Key::A) {
            actions.push(Action::SelectAll);
        }
        if cmd && key(egui::Key::C) {
            actions.push(Action::Copy);
        }
        if cmd && key(egui::Key::X) {
            actions.push(Action::Cut);
        }
        if cmd && key(egui::Key::V) {
            actions.push(Action::Paste);
        }
        if cmd && shift && key(egui::Key::N) {
            actions.push(Action::NewFolder);
        }
        if cmd && key(egui::Key::T) {
            actions.push(Action::NewTab);
        }
        if cmd && key(egui::Key::W) {
            actions.push(Action::CloseTab(self.active));
        }
        if cmd && key(egui::Key::D) {
            actions.push(Action::ToggleTheme);
        }
        if cmd && key(egui::Key::Comma) {
            actions.push(Action::OpenSettings);
        }
        if cmd && key(egui::Key::E) {
            actions.push(Action::ToggleEditMode);
        }
        if cmd && key(egui::Key::F) {
            actions.push(Action::FocusSearch);
        }
        if mods.alt && key(egui::Key::ArrowLeft) {
            actions.push(Action::Back);
        }
        if mods.alt && key(egui::Key::ArrowRight) {
            actions.push(Action::Forward);
        }
        if mods.alt && key(egui::Key::ArrowUp) {
            actions.push(Action::Up);
        }
        if key(egui::Key::F5) {
            actions.push(Action::Refresh);
        }
        if key(egui::Key::F2) {
            if let Some(entry) = self.active_tab().selected_entries().first() {
                actions.push(Action::RenameStart(entry.path.clone()));
            }
        }
        if key(egui::Key::Delete) {
            let paths = self.active_tab().selected_paths();
            if !paths.is_empty() {
                actions.push(Action::DeleteRequest(paths, shift));
            }
        }
        if key(egui::Key::Enter) {
            let selected = self.active_tab().selected_entries();
            if !selected.is_empty() {
                actions.push(Action::Open(selected.into_iter().map(|e| e.path).collect()));
            }
        }
        if key(egui::Key::Backspace) && !cmd && !mods.alt {
            actions.push(Action::Up);
        }
        actions
    }
}

// ---------------------------------------------------------------------------
// eframe integration
// ---------------------------------------------------------------------------

impl eframe::App for ExplorerApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let c = self.palette.window;
        [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, 1.0]
    }

    /// Called once when the window is closed: keeps the user's window size.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_prefs();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.frame_time = ctx.input(|i| i.time);
        self.track_window_size(&ctx);
        if self.mode == ThemeMode::System && self.frame_time - self.last_system_check > 2.5 {
            self.last_system_check = self.frame_time;
            self.system_dark = theme::system_uses_dark();
        }
        let resolved = self.mode.resolve(self.system_dark);
        if resolved != self.applied_mode
            || self.accent != self.applied_accent
            || self.custom_accent != self.applied_custom
        {
            self.palette = palette_with(resolved, self.accent, self.custom_accent);
            theme::apply_style(&ctx, &self.palette);
            self.applied_mode = resolved;
            self.applied_accent = self.accent;
            self.applied_custom = self.custom_accent;
        }
        self.icons.poll(&ctx);
        self.poll_search(&ctx);
        self.poll_special();

        // Window title follows the active tab, like Explorer.
        let title = f(t("{} — Проводник"), &[&self.active_tab().title()]);
        if title != self.last_title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }

        let p = self.palette;
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, CornerRadius::ZERO, p.window);

        let mut actions = self.handle_shortcuts(&ctx);

        let tab_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), TAB_BAR_H));
        let command_rect = Rect::from_min_size(
            Pos2::new(rect.left(), tab_rect.bottom()),
            Vec2::new(rect.width(), COMMAND_BAR_H),
        );
        let address_rect = Rect::from_min_size(
            Pos2::new(rect.left(), command_rect.bottom()),
            Vec2::new(rect.width(), ADDRESS_BAR_H),
        );
        let status_rect = Rect::from_min_size(
            Pos2::new(rect.left(), rect.bottom() - STATUS_BAR_H),
            Vec2::new(rect.width(), STATUS_BAR_H),
        );
        let body_rect = Rect::from_min_max(
            Pos2::new(rect.left(), address_rect.bottom()),
            Pos2::new(rect.right(), status_rect.top()),
        );

        actions.extend(ui_chrome::tab_bar(self, ui, tab_rect));
        actions.extend(ui_chrome::command_bar(self, ui, command_rect));
        actions.extend(ui_chrome::address_bar(self, ui, address_rect));

        // Navigation pane + content card.
        let nav_width = self.nav_width.clamp(NAV_MIN, NAV_MAX).min(body_rect.width() - 320.0);
        let nav_rect = Rect::from_min_size(
            Pos2::new(body_rect.left(), body_rect.top()),
            Vec2::new(nav_width, body_rect.height()),
        );
        let content_rect = Rect::from_min_max(
            Pos2::new(body_rect.left() + nav_width + 6.0, body_rect.top() + 4.0),
            Pos2::new(body_rect.right() - 8.0, body_rect.bottom() - 6.0),
        );

        actions.extend(ui_nav::show(self, ui, nav_rect));

        // Splitter.
        let splitter = Rect::from_min_size(
            Pos2::new(body_rect.left() + nav_width, body_rect.top()),
            Vec2::new(6.0, body_rect.height()),
        );
        let split_resp = ui.interact(splitter, Id::new("nav-splitter"), Sense::drag());
        if split_resp.hovered() || split_resp.dragged() {
            ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
            ui.painter().rect_filled(
                Rect::from_min_size(
                    Pos2::new(splitter.center().x - 1.0, body_rect.top() + 20.0),
                    Vec2::new(2.0, body_rect.height() - 40.0),
                ),
                CornerRadius::same(1),
                p.border_strong,
            );
        }
        if split_resp.dragged() {
            self.nav_width = (self.nav_width + split_resp.drag_delta().x)
                .clamp(NAV_MIN, (NAV_MAX).min(body_rect.width() - 340.0));
        }

        // Content card.
        let shadow = egui::epaint::Shadow {
            offset: [0, 1],
            blur: 3,
            spread: 0,
            color: if p.dark {
                Color32::from_black_alpha(60)
            } else {
                Color32::from_black_alpha(18)
            },
        };
        ui.painter().add(shadow.as_shape(content_rect, CornerRadius::same(8)));
        ui.painter().rect(
            content_rect,
            CornerRadius::same(8),
            p.content,
            Stroke::new(1.0, p.border),
            egui::StrokeKind::Inside,
        );
        {
            let child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(content_rect.shrink(1.0))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            let mut child = child;
            child.set_clip_rect(content_rect.shrink(1.0));
            actions.extend(ui_files::show(self, &mut child, content_rect.shrink(1.0)));
        }

        actions.extend(ui_chrome::status_bar(self, ui, status_rect));

        // Drag & drop from Windows Explorer.
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| Some(f.path().to_path_buf()))
                .collect()
        });
        if !dropped.is_empty() {
            if let Some(dir) = self.paste_dir() {
                match fs::copy_into(&dir, &dropped, false) {
                    Ok(count) => {
                        self.active_tab_mut().reload();
                        self.toast(f(t("Скопировано элементов: {}"), &[&count.to_string()]));
                    }
                    Err(err) => {
                        self.dialog = Dialog::Error {
                            title: t("Не удается скопировать элементы").into(),
                            message: err,
                        };
                    }
                }
            }
        }

        self.edge_resize(ui, &ctx);

        if self.menu.is_some() && ui.input(|i| !i.pointer.any_down()) {
            // Keep the menu alive; dismissal is handled in show_menu.
        }
        self.show_menu(&ctx);
        actions.extend(dialogs::show(self, &ctx));
        dialogs::toasts(self, &ctx);

        for action in actions {
            self.apply(action, &ctx);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn error_text(err: &std::io::Error) -> String {
    match err.raw_os_error() {
        Some(5) => t("Отказано в доступе.").into(),
        Some(2) => t("Не удается найти указанный файл.").into(),
        Some(3) => t("Не удается найти указанный путь.").into(),
        Some(32) => t("Файл занят другим процессом.").into(),
        Some(145) => t("Папка не пуста.").into(),
        Some(206) => t("Слишком длинное имя файла или путь.").into(),
        _ => match err.kind() {
            std::io::ErrorKind::PermissionDenied => t("Отказано в доступе.").into(),
            std::io::ErrorKind::NotFound => t("Не удается найти указанный файл.").into(),
            std::io::ErrorKind::AlreadyExists => t("Файл с таким именем уже существует.").into(),
            _ => {
                let text = err.to_string();
                if text.trim().is_empty() {
                    t("Неизвестная ошибка.").into()
                } else {
                    text
                }
            }
        },
    }
}

fn initial_location() -> Location {
    if let Some(arg) = std::env::args().nth(1) {
        let path = PathBuf::from(arg);
        if path.is_dir() {
            return Location::Dir(path);
        }
        if let Some(parent) = path.parent() {
            if parent.is_dir() {
                return Location::Dir(parent.to_path_buf());
            }
        }
    }
    Location::Home
}

fn collect_recent() -> Vec<Entry> {
    let roots: Vec<PathBuf> = [
        dirs::desktop_dir(),
        dirs::download_dir(),
        dirs::document_dir(),
        dirs::picture_dir(),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut files: Vec<Entry> = Vec::new();
    for root in roots {
        collect_files(&root, 0, &mut files, 4000);
    }
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    files.truncate(14);
    files
}

fn collect_files(dir: &Path, depth: usize, out: &mut Vec<Entry>, limit: usize) {
    if out.len() >= limit || depth > 2 {
        return;
    }
    let Ok(read) = std::fs::read_dir(dir) else { return };
    for item in read.flatten().take(600) {
        if out.len() >= limit {
            return;
        }
        // Recent-file scans skip Windows metadata files as well.
        if is_meta_name(&item.file_name()) {
            continue;
        }
        let Ok(meta) = item.metadata() else { continue };
        if meta.is_dir() {
            if depth < 2 {
                let name = item.file_name().to_string_lossy().to_string();
                if !name.starts_with('.') && name != "node_modules" && name != "target" {
                    collect_files(&item.path(), depth + 1, out, limit);
                }
            }
        } else {
            let entry = Entry::from_path(item.path(), false, &meta);
            if !entry.hidden {
                out.push(entry);
            }
        }
    }
}

fn collect_gallery() -> Vec<Entry> {
    let Some(pictures) = fs::pictures_dir() else { return Vec::new() };
    let mut images = Vec::new();
    collect_images(&pictures, 0, &mut images, 900);
    images.sort_by(|a, b| b.modified.cmp(&a.modified));
    images
}

fn collect_images(dir: &Path, depth: usize, out: &mut Vec<Entry>, limit: usize) {
    if out.len() >= limit || depth > 5 {
        return;
    }
    let Ok(read) = std::fs::read_dir(dir) else { return };
    for item in read.flatten().take(1000) {
        if out.len() >= limit {
            return;
        }
        if is_meta_name(&item.file_name()) {
            continue;
        }
        let Ok(meta) = item.metadata() else { continue };
        if meta.is_dir() {
            let name = item.file_name().to_string_lossy().to_string();
            if !name.starts_with('.') {
                collect_images(&item.path(), depth + 1, out, limit);
            }
        } else {
            let entry = Entry::from_path(item.path(), false, &meta);
            if entry.is_image() && !entry.hidden {
                out.push(entry);
            }
        }
    }
}

/// Windows metadata files that are never shown to the user as normal files.
fn is_meta_name(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_ascii_lowercase().to_str(),
        Some("desktop.ini") | Some("$recycle.bin")
    )
}

/// Paints text vertically centred in a rect, ellipsised to the available width.
pub fn paint_ellipsised(
    ui: &Ui,
    text: &str,
    rect: Rect,
    font: egui::FontId,
    color: Color32,
) {
    let galley = theme::ellipsis_galley(ui, text, font, color, rect.width().max(8.0));
    let pos = Pos2::new(rect.left(), rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(pos, galley, color);
}
