//! The window: a strip of groups, and under the current group a strip of terminals.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

use gio::prelude::*;
use gtk::gdk::{self, EventType};
use gtk::glib::{self, ControlFlow, Propagation};
use gtk::pango;
use gtk::prelude::*;
use vte::TerminalExt;

use crate::paths::{display_path, existing_directory, folder_uri, home, new_id, pid_cwd};
use crate::session::{self, GroupRecord, Session, TabRecord};
use crate::shell::{self, tab_environment};
use crate::style::{self, TerminalLook};

const POLL_MS: u64 = 700;
const SAVE_MS: u64 = 400;

struct Group {
    id: String,
    name: RefCell<String>,
    named: bool,
    notebook: gtk::Notebook,
    label: gtk::Label,
    tab: gtk::EventBox,
    tabs: RefCell<Vec<Rc<Tab>>>,
}

struct Tab {
    id: String,
    cwd: RefCell<String>,
    title: RefCell<String>,
    group: RefCell<Weak<Group>>,
    history: PathBuf,
    pid: Cell<Option<i32>>,
    spawned: Cell<bool>,
    closing: Cell<bool>,
    terminal: vte::Terminal,
    label: gtk::Label,
    page: gtk::ScrolledWindow,
    header: gtk::EventBox,
}

pub struct Ui {
    pub window: gtk::ApplicationWindow,
    app: gtk::Application,
    header: gtk::HeaderBar,
    open_button: gtk::Button,
    groups_notebook: gtk::Notebook,
    groups: Vec<Rc<Group>>,
    look: TerminalLook,
    quitting: bool,
    save_scheduled: bool,
    profile_settings: Option<gio::Settings>,
    profile_handler: Option<glib::SignalHandlerId>,
    bench: bool,
}

pub fn open_window(app: &gtk::Application, bench: bool) -> Rc<RefCell<Ui>> {
    let window = gtk::ApplicationWindow::new(app);
    window.set_title("Multiterm");
    window.set_default_size(1000, 640);
    let icon = crate::paths::install_app_icon();
    let _ = window.set_icon_from_file(&icon);
    let _ = gtk::Window::set_default_icon_from_file(&icon);
    let header = gtk::HeaderBar::new();
    header.set_show_close_button(true);
    header.set_title(Some("Multiterm"));
    let new_tab = icon_button("tab-new-symbolic", "New tab (Ctrl+Shift+T)");
    let open_button = icon_button("folder-open-symbolic", "Open folder in the file manager (Ctrl+Shift+O)");
    let new_group = icon_button("list-add-symbolic", "New group (Ctrl+Shift+G)");
    let preferences = icon_button("preferences-system-symbolic", "Preferences");
    header.pack_start(&new_tab);
    header.pack_start(&open_button);
    header.pack_start(&new_group);
    header.pack_end(&preferences);
    window.set_titlebar(Some(&header));

    let groups_notebook = gtk::Notebook::new();
    groups_notebook.set_show_tabs(false);
    groups_notebook.set_show_border(false);
    groups_notebook.set_scrollable(true);
    groups_notebook.style_context().add_class("groups");
    window.add(&groups_notebook);
    install_css();

    let ui = Rc::new(RefCell::new(Ui {
        window: window.clone(),
        app: app.clone(),
        header,
        open_button: open_button.clone(),
        groups_notebook: groups_notebook.clone(),
        groups: Vec::new(),
        look: style::load_terminal_look(),
        quitting: false,
        save_scheduled: false,
        profile_settings: None,
        profile_handler: None,
        bench,
    }));

    let ui_new = ui.clone();
    new_tab.connect_clicked(move |_| new_tab_action(&ui_new));
    let ui_open = ui.clone();
    open_button.connect_clicked(move |_| open_current_folder(&ui_open));
    let ui_group = ui.clone();
    new_group.connect_clicked(move |_| {
        new_group_action(&ui_group, None);
    });
    let ui_prefs = ui.clone();
    preferences.connect_clicked(move |_| {
        let window = ui_prefs.borrow().window.clone();
        crate::preferences::open_preferences(window.upcast_ref());
    });

    bind_actions(&ui);
    let ui_close = ui.clone();
    window.connect_delete_event(move |_, _| {
        ui_close.borrow_mut().quitting = true;
        save_now(&ui_close);
        Propagation::Proceed
    });
    let ui_size = ui.clone();
    window.connect_configure_event(move |_, _| {
        schedule_save(&ui_size);
        false
    });

    groups_notebook.connect_switch_page({
        let ui = ui.clone();
        move |_, _, _| {
            glib::idle_add_local({
                let ui = ui.clone();
                move || {
                    refresh_header(&ui);
                    ControlFlow::Break
                }
            });
            schedule_save(&ui);
        }
    });

    restore_or_start(&ui);
    watch_settings(&ui);
    glib::timeout_add_local(Duration::from_millis(POLL_MS), {
        let ui = ui.clone();
        move || poll_directories(&ui)
    });
    glib::idle_add_local({
        let ui = ui.clone();
        move || {
            if let Some(tab) = current_tab(&ui) {
                tab.terminal.grab_focus();
            }
            if ui.borrow().bench {
                start_bench(&ui);
            }
            ControlFlow::Break
        }
    });
    ui
}

fn install_css() {
    let Some(screen) = gdk::Screen::default() else {
        return;
    };
    let css = gtk::CssProvider::new();
    let _ = css.load_from_data(
        b"notebook tab { min-width: 180px; } notebook.groups tab { min-width: 140px; }",
    );
    gtk::StyleContext::add_provider_for_screen(
        &screen,
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn bind_actions(ui: &Rc<RefCell<Ui>>) {
    add_action(ui, "new-tab", &["<Shift><Control>t"], |ui| new_tab_action(ui));
    add_action(ui, "close-tab", &["<Shift><Control>w"], |ui| {
        if let Some(tab) = current_tab(ui) {
            close_tab(ui, &tab);
        }
    });
    add_action(ui, "open-folder", &["<Shift><Control>o"], open_current_folder);
    add_action(ui, "new-group", &["<Shift><Control>g"], |ui| {
        new_group_action(ui, None);
    });
    add_action(ui, "copy", &["<Shift><Control>c"], |ui| {
        if let Some(tab) = current_tab(ui) {
            tab.terminal.copy_clipboard_format(vte::Format::Text);
        }
    });
    add_action(ui, "paste", &["<Shift><Control>v"], |ui| {
        if let Some(tab) = current_tab(ui) {
            tab.terminal.paste_clipboard();
        }
    });
    add_action(ui, "quit", &["<Shift><Control>q"], |ui| ui.borrow().window.close());
    add_action(ui, "prev-tab", &["<Control>Page_Up"], |ui| {
        if let Some(group) = current_group(ui) {
            group.notebook.prev_page();
        }
    });
    add_action(ui, "next-tab", &["<Control>Page_Down"], |ui| {
        if let Some(group) = current_group(ui) {
            group.notebook.next_page();
        }
    });
    for index in 1..=9 {
        let action = format!("tab-{index}");
        let app = ui.borrow().app.clone();
        let window = ui.borrow().window.clone();
        let ui = ui.clone();
        let simple = gio::SimpleAction::new(&action, None);
        simple.connect_activate(move |_, _| focus_tab(&ui, index - 1));
        window.add_action(&simple);
        app.set_accels_for_action(&format!("win.{action}"), &[&format!("<Alt>{index}")]);
    }
}

fn add_action(ui: &Rc<RefCell<Ui>>, name: &str, accels: &[&str], callback: impl Fn(&Rc<RefCell<Ui>>) + 'static) {
    let app = ui.borrow().app.clone();
    let window = ui.borrow().window.clone();
    let ui = ui.clone();
    let action = gio::SimpleAction::new(name, None);
    action.connect_activate(move |_, _| callback(&ui));
    window.add_action(&action);
    app.set_accels_for_action(&format!("win.{name}"), accels);
}

fn restore_or_start(ui: &Rc<RefCell<Ui>>) {
    let session = session::load_session();
    let mut keep = HashSet::new();
    let mut active_index = 0i64;
    if let Some(session) = session.as_ref() {
        if let Some((width, height)) = session.window_size() {
            ui.borrow().window.set_default_size(width, height);
        }
        for record in &session.groups {
            let group = add_group(ui, &record.name, Some(record.id.clone()), record.named);
            for tab in &record.tabs {
                keep.insert(tab.id.clone());
                add_tab(ui, &tab.cwd, Some(tab.id.clone()), &tab.title, Some(group.clone()), false);
            }
            let pages = group.notebook.n_pages();
            if pages > 0 {
                let active = if (0..pages as i64).contains(&record.active) { record.active as u32 } else { 0 };
                group.notebook.set_current_page(Some(active));
            }
        }
        active_index = session.active_group;
        let all_named = ui.borrow().groups.iter().all(|group| group.named) && !ui.borrow().groups.is_empty();
        if all_named {
            let loose = add_group(ui, "", None, false);
            move_group(ui, &loose, 0);
            active_index += 1;
        }
    }
    if ui.borrow().groups.is_empty() {
        add_group(ui, "", None, false);
    }
    if all_tabs(ui).is_empty() {
        let tab = add_tab(ui, &home().to_string_lossy(), None, "", None, true);
        keep = HashSet::from([tab.id.clone()]);
    } else if !ui.borrow().groups.is_empty() {
        let count = ui.borrow().groups.len() as i64;
        if active_index < 0 || active_index >= count {
            active_index = 0;
        }
        let group = ui.borrow().groups[active_index as usize].clone();
        show_group(ui, &group);
    }
    session::gc_history(&keep);
    refresh_labels(ui);
    refresh_header(ui);
    sync_group_tabs(ui);
}

fn add_group(ui: &Rc<RefCell<Ui>>, name: &str, id: Option<String>, named: bool) -> Rc<Group> {
    let label = gtk::Label::new(Some(&display_group_name(name, named)));
    label.set_margin_start(8);
    label.set_margin_end(8);
    let tab = gtk::EventBox::new();
    tab.add(&label);
    tab.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    let group = Rc::new(Group {
        id: id.unwrap_or_else(new_id),
        name: RefCell::new(name.to_string()),
        named,
        notebook: gtk::Notebook::new(),
        label,
        tab,
        tabs: RefCell::new(Vec::new()),
    });
    group.notebook.set_scrollable(true);
    group.notebook.set_show_border(false);
    group.notebook.connect_switch_page({
        let ui = ui.clone();
        move |_, _, _| {
            glib::idle_add_local({
                let ui = ui.clone();
                move || {
                    refresh_header(&ui);
                    ControlFlow::Break
                }
            });
            schedule_save(&ui);
        }
    });
    group.notebook.connect_page_reordered({
        let ui = ui.clone();
        move |_, _, _| schedule_save(&ui)
    });
    group.tab.connect_button_press_event({
        let ui = ui.clone();
        let group = Rc::downgrade(&group);
        move |_, event| {
            let Some(group) = group.upgrade() else {
                return Propagation::Proceed;
            };
            if !group.named {
                return Propagation::Proceed;
            }
            if event.event_type() == EventType::DoubleButtonPress && event.button() == 1 {
                rename_group(&ui, &group);
                Propagation::Stop
            } else if event.button() == 3 && event.event_type() == EventType::ButtonPress {
                popup_group_menu(&ui, &group);
                Propagation::Stop
            } else {
                Propagation::Proceed
            }
        }
    });
    enable_group_drop(ui, &group);
    let notebook = {
        let mut state = ui.borrow_mut();
        state.groups.push(group.clone());
        state.groups_notebook.clone()
    };
    notebook.append_page(&group.notebook, Some(&group.tab));
    notebook.set_tab_reorderable(&group.notebook, false);
    notebook.set_tab_detachable(&group.notebook, false);
    group.notebook.show_all();
    group.tab.show_all();
    sync_group_tabs(ui);
    schedule_save(ui);
    group
}

fn new_group_action(ui: &Rc<RefCell<Ui>>, tab: Option<Rc<Tab>>) -> Rc<Group> {
    let tab = tab.or_else(|| current_tab(ui));
    let name = next_group_name(ui);
    let group = add_group(ui, &name, None, true);
    if let Some(tab) = tab {
        move_tab_to_group(ui, &tab, &group, true);
    }
    show_group(ui, &group);
    group
}

fn set_group_name(ui: &Rc<RefCell<Ui>>, group: &Group, name: &str) {
    let cleaned = name.trim();
    if !group.named || cleaned.is_empty() {
        return;
    }
    *group.name.borrow_mut() = cleaned.to_string();
    group.label.set_text(&display_group_name(cleaned, true));
    schedule_save(ui);
}

fn ungroup(ui: &Rc<RefCell<Ui>>, group: &Rc<Group>) {
    if !group.named || !ui.borrow().groups.iter().any(|item| item.id == group.id) {
        return;
    }
    let loose = loose_group(ui);
    for tab in group.tabs.borrow().clone() {
        move_tab_to_group(ui, &tab, &loose, false);
    }
    remove_group(ui, group);
    show_group(ui, &loose);
}

fn close_group_terminals(ui: &Rc<RefCell<Ui>>, group: &Rc<Group>) {
    for tab in group.tabs.borrow().clone() {
        if ui.borrow().quitting {
            return;
        }
        close_tab(ui, &tab);
        if ui.borrow().quitting {
            return;
        }
    }
    if group.named && group.notebook.n_pages() == 0 && ui.borrow().groups.iter().any(|item| item.id == group.id) {
        remove_group(ui, group);
    }
}

fn add_tab(
    ui: &Rc<RefCell<Ui>>,
    cwd: &str,
    id: Option<String>,
    title: &str,
    group: Option<Rc<Group>>,
    focus: bool,
) -> Rc<Tab> {
    let group = group.unwrap_or_else(|| active_group(ui));
    let id = id.unwrap_or_else(new_id);
    let history = session::history_path(&id);
    shell::prepare_history(&history);
    let cwd = existing_directory(cwd);
    let label = gtk::Label::new(Some(&display_path(&cwd)));
    label.set_halign(gtk::Align::Start);
    label.set_xalign(0.0);
    label.set_ellipsize(pango::EllipsizeMode::Middle);
    label.set_margin_start(8);
    label.set_margin_end(8);
    let terminal = vte::Terminal::new();
    terminal.set_hexpand(true);
    terminal.set_vexpand(true);
    let page = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    page.set_hexpand(true);
    page.set_vexpand(true);
    page.add(&terminal);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    row.pack_start(&label, true, true, 0);
    let close = icon_button("window-close-symbolic", "Close tab");
    close.style_context().add_class("flat");
    row.pack_end(&close, false, false, 0);
    let header = gtk::EventBox::new();
    header.add(&row);
    header.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    let tab = Rc::new(Tab {
        id,
        cwd: RefCell::new(cwd),
        title: RefCell::new(title.trim().to_string()),
        group: RefCell::new(Rc::downgrade(&group)),
        history,
        pid: Cell::new(None),
        spawned: Cell::new(false),
        closing: Cell::new(false),
        terminal,
        label,
        page,
        header,
    });
    configure_terminal(ui, &tab);
    apply_scrollbar(ui, &tab);
    let position = insert_position(ui, &group, focus);
    group.notebook.insert_page(&tab.page, Some(&tab.header), Some(position));
    group.notebook.set_tab_reorderable(&tab.page, true);
    group.notebook.set_tab_detachable(&tab.page, false);
    group.tabs.borrow_mut().push(tab.clone());
    tab.page.show_all();
    tab.header.show_all();
    let ui_close = ui.clone();
    let tab_close = Rc::downgrade(&tab);
    close.connect_clicked(move |_| {
        if let Some(tab) = tab_close.upgrade() {
            close_tab(&ui_close, &tab);
        }
    });
    install_tab_gestures(ui, &tab);
    if focus {
        show_group(ui, &group);
        if let Some(index) = group.notebook.page_num(&tab.page) {
            group.notebook.set_current_page(Some(index));
        }
        tab.terminal.grab_focus();
    }
    if tab.terminal.is_realized() {
        spawn_tab(ui, &tab);
    }
    refresh_labels(ui);
    schedule_save(ui);
    tab
}

fn new_tab_action(ui: &Rc<RefCell<Ui>>) {
    let cwd = if ui.borrow().look.preserve_directory == "never" {
        home().to_string_lossy().into_owned()
    } else if let Some(tab) = current_tab(ui).or_else(|| all_tabs(ui).pop()) {
        tab.cwd.borrow().clone()
    } else {
        home().to_string_lossy().into_owned()
    };
    add_tab(ui, &cwd, None, "", None, true);
}

fn close_tab(ui: &Rc<RefCell<Ui>>, tab: &Rc<Tab>) {
    if ui.borrow().quitting || tab.closing.get() {
        return;
    }
    tab.closing.set(true);
    if let Some(group) = tab.group.borrow().upgrade() {
        if let Some(index) = group.notebook.page_num(&tab.page) {
            group.notebook.remove_page(Some(index));
        }
        group.tabs.borrow_mut().retain(|item| item.id != tab.id);
    }
    save_now(ui);
    let _ = std::fs::remove_file(&tab.history);
    let _ = std::fs::remove_file(session::ready_path(&tab.history));
    if all_tabs(ui).is_empty() {
        ui.borrow_mut().quitting = true;
        ui.borrow().window.close();
        return;
    }
    refresh_labels(ui);
    refresh_header(ui);
}

fn focus_tab(ui: &Rc<RefCell<Ui>>, index: i32) {
    let Some(group) = current_group(ui) else {
        return;
    };
    if index < 0 || index as u32 >= group.notebook.n_pages() {
        return;
    }
    group.notebook.set_current_page(Some(index as u32));
    if let Some(tab) = current_tab(ui) {
        tab.terminal.grab_focus();
    }
}

fn move_tab_to_group(ui: &Rc<RefCell<Ui>>, tab: &Rc<Tab>, group: &Rc<Group>, focus: bool) {
    if let Some(old) = tab.group.borrow().upgrade() {
        if old.id == group.id {
            return;
        }
        if let Some(index) = old.notebook.page_num(&tab.page) {
            old.notebook.remove_page(Some(index));
        }
        old.tabs.borrow_mut().retain(|item| item.id != tab.id);
    }
    *tab.group.borrow_mut() = Rc::downgrade(group);
    let position = insert_position(ui, group, focus);
    group.notebook.insert_page(&tab.page, Some(&tab.header), Some(position));
    group.notebook.set_tab_reorderable(&tab.page, true);
    group.notebook.set_tab_detachable(&tab.page, false);
    group.tabs.borrow_mut().push(tab.clone());
    tab.page.show_all();
    tab.header.show_all();
    if focus {
        show_group(ui, group);
        if let Some(index) = group.notebook.page_num(&tab.page) {
            group.notebook.set_current_page(Some(index));
        }
    }
    schedule_save(ui);
}

fn open_current_folder(ui: &Rc<RefCell<Ui>>) {
    let Some(tab) = current_tab(ui) else {
        return;
    };
    let cwd = pid_cwd(tab.pid.get()).unwrap_or_else(|| tab.cwd.borrow().clone());
    open_folder(&cwd);
}

fn open_folder(path: &str) {
    let _ = gio::AppInfo::launch_default_for_uri(&folder_uri(path), None::<&gio::AppLaunchContext>);
}

fn configure_terminal(ui: &Rc<RefCell<Ui>>, tab: &Rc<Tab>) {
    let look = ui.borrow().look.clone();
    style::apply_terminal_look(&tab.terminal, &look);
    tab.terminal.connect_child_exited({
        let ui = ui.clone();
        let tab = Rc::downgrade(tab);
        move |_, _status| {
            let Some(tab) = tab.upgrade() else {
                return;
            };
            if ui.borrow().quitting || tab.closing.get() {
                return;
            }
            let action = ui.borrow().look.exit_action.clone();
            if action == "hold" {
                return;
            }
            if action == "restart" {
                tab.spawned.set(false);
                let ui = ui.clone();
                glib::idle_add_local(move || {
                    spawn_tab(&ui, &tab);
                    ControlFlow::Break
                });
                return;
            }
            let ui = ui.clone();
            let tab = tab.clone();
            glib::idle_add_local(move || {
                close_tab(&ui, &tab);
                ControlFlow::Break
            });
        }
    });
    tab.terminal.connect_realize({
        let ui = ui.clone();
        let tab = Rc::downgrade(tab);
        move |_| {
            if let Some(tab) = tab.upgrade() {
                spawn_tab(&ui, &tab);
            }
        }
    });
    tab.terminal.connect_button_press_event({
        let ui = ui.clone();
        let tab = Rc::downgrade(tab);
        move |_, event| {
            if event.button() == 3 && event.event_type() == EventType::ButtonPress {
                if let Some(tab) = tab.upgrade() {
                    popup_tab_menu(&ui, &tab);
                }
                Propagation::Stop
            } else {
                Propagation::Proceed
            }
        }
    });
}

fn spawn_tab(ui: &Rc<RefCell<Ui>>, tab: &Rc<Tab>) {
    if tab.spawned.get() || tab.closing.get() {
        return;
    }
    tab.spawned.set(true);
    let cwd = existing_directory(&tab.cwd.borrow());
    *tab.cwd.borrow_mut() = cwd.clone();
    let argv = shell_argv(&ui.borrow().look);
    let env = tab_environment(&tab.history);
    let argv_paths: Vec<PathBuf> = argv.iter().map(PathBuf::from).collect();
    let env_paths: Vec<PathBuf> = env.iter().map(PathBuf::from).collect();
    let argv_refs: Vec<&Path> = argv_paths.iter().map(PathBuf::as_path).collect();
    let env_refs: Vec<&Path> = env_paths.iter().map(PathBuf::as_path).collect();
    let terminal = tab.terminal.clone();
    let spawned = tab.clone();
    terminal.spawn_async(
        vte::PtyFlags::DEFAULT,
        Some(&cwd),
        &argv_refs,
        &env_refs,
        glib::SpawnFlags::DEFAULT,
        Some(Box::new(|| {})),
        -1,
        None::<&gio::Cancellable>,
        Some(Box::new(move |_terminal, pid, error| {
            if let Some(error) = error {
                eprintln!("Could not start the shell: {error}");
            } else {
                spawned.pid.set(Some(pid.0));
            }
        })),
    );
}

fn shell_argv(look: &TerminalLook) -> Vec<String> {
    if look.use_custom_command {
        let command = look.custom_command.trim();
        if !command.is_empty() {
            if let Ok(parsed) = glib::shell_parse_argv(command) {
                let argv: Vec<String> = parsed.into_iter().map(|item| item.to_string_lossy().into_owned()).collect();
                if !argv.is_empty() {
                    return argv;
                }
            }
        }
    }
    let init = shell::shell_init_path().display().to_string();
    if look.login_shell {
        vec!["/bin/bash".into(), "--login".into(), "--rcfile".into(), init]
    } else {
        vec!["/bin/bash".into(), "--rcfile".into(), init]
    }
}

fn apply_scrollbar(ui: &Rc<RefCell<Ui>>, tab: &Tab) {
    let policy = ui.borrow().look.scrollbar_policy.clone();
    let (vertical, overlay) = style::scrollbar_policy(&policy);
    tab.page.set_policy(gtk::PolicyType::Never, vertical);
    tab.page.set_overlay_scrolling(overlay);
}

fn install_tab_gestures(ui: &Rc<RefCell<Ui>>, tab: &Rc<Tab>) {
    tab.header.connect_button_press_event({
        let ui = ui.clone();
        let tab = Rc::downgrade(tab);
        move |_, event| {
            let Some(tab) = tab.upgrade() else {
                return Propagation::Proceed;
            };
            if event.event_type() == EventType::DoubleButtonPress && event.button() == 1 {
                rename_tab(&ui, &tab);
                Propagation::Stop
            } else if event.button() == 3 && event.event_type() == EventType::ButtonPress {
                popup_tab_menu(&ui, &tab);
                Propagation::Stop
            } else {
                Propagation::Proceed
            }
        }
    });
    let targets = [gtk::TargetEntry::new("multiterm-tab", gtk::TargetFlags::SAME_APP, 0)];
    tab.header.drag_source_set(gdk::ModifierType::BUTTON1_MASK, &targets, gdk::DragAction::MOVE);
    let id = tab.id.clone();
    tab.header.connect_drag_data_get(move |_, _, data, _, _| {
        let _ = data.set_text(&id);
    });
}

fn enable_group_drop(ui: &Rc<RefCell<Ui>>, group: &Rc<Group>) {
    let targets = [gtk::TargetEntry::new("multiterm-tab", gtk::TargetFlags::SAME_APP, 0)];
    group.tab.drag_dest_set(gtk::DestDefaults::ALL, &targets, gdk::DragAction::MOVE);
    group.tab.connect_drag_data_received({
        let ui = ui.clone();
        let group = Rc::downgrade(group);
        move |_, _, _, _, data, _, _| {
            let Some(id) = data.text() else {
                return;
            };
            let Some(group) = group.upgrade() else {
                return;
            };
            let Some(tab) = tab_by_id(&ui, id.as_str()) else {
                return;
            };
            move_tab_to_group(&ui, &tab, &group, true);
        }
    });
}

fn popup_menu(parent: &impl IsA<gtk::Widget>, items: Vec<(String, Box<dyn Fn()>)>) {
    let popover = gtk::Popover::new(Some(parent));
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);
    for (label, action) in items {
        let button = gtk::Button::with_label(&label);
        button.style_context().add_class("flat");
        let popover = popover.clone();
        button.connect_clicked(move |_| {
            action();
            popover.popdown();
        });
        box_.pack_start(&button, false, false, 0);
    }
    popover.add(&box_);
    box_.show_all();
    popover.popup();
}

fn popup_tab_menu(ui: &Rc<RefCell<Ui>>, tab: &Rc<Tab>) {
    let mut items: Vec<(String, Box<dyn Fn()>)> = Vec::new();
    let ui_rename = ui.clone();
    let tab_rename = tab.clone();
    items.push(("Rename".into(), Box::new(move || rename_tab(&ui_rename, &tab_rename))));
    let ui_group = ui.clone();
    let tab_group = tab.clone();
    items.push(("New Group".into(), Box::new(move || {
        new_group_action(&ui_group, Some(tab_group.clone()));
    })));
    let named: Vec<Rc<Group>> = ui.borrow().groups.iter().filter(|group| group.named).cloned().collect();
    let current = tab.group.borrow().upgrade().map(|group| group.id.clone());
    if !named.is_empty() {
        for group in ui.borrow().groups.clone() {
            if Some(group.id.clone()) == current {
                continue;
            }
            let label = format!("Move to {}", display_group_name(&group.name.borrow(), group.named));
            let ui_move = ui.clone();
            let tab_move = tab.clone();
            items.push((label, Box::new(move || move_tab_to_group(&ui_move, &tab_move, &group, true))));
        }
    }
    let tab_copy = tab.clone();
    items.push(("Copy".into(), Box::new(move || {
        tab_copy.terminal.copy_clipboard_format(vte::Format::Text);
    })));
    let tab_paste = tab.clone();
    items.push(("Paste".into(), Box::new(move || tab_paste.terminal.paste_clipboard())));
    let tab_folder = tab.clone();
    items.push(("Open Folder".into(), Box::new(move || {
        let cwd = pid_cwd(tab_folder.pid.get()).unwrap_or_else(|| tab_folder.cwd.borrow().clone());
        open_folder(&cwd);
    })));
    let ui_close = ui.clone();
    let tab_close = tab.clone();
    items.push(("Close Tab".into(), Box::new(move || close_tab(&ui_close, &tab_close))));
    popup_menu(&tab.header, items);
}

fn popup_group_menu(ui: &Rc<RefCell<Ui>>, group: &Rc<Group>) {
    let ui_rename = ui.clone();
    let group_rename = group.clone();
    let ui_ungroup = ui.clone();
    let group_ungroup = group.clone();
    let ui_close = ui.clone();
    let group_close = group.clone();
    popup_menu(
        &group.tab,
        vec![
            ("Rename".into(), Box::new(move || rename_group(&ui_rename, &group_rename))),
            ("Ungroup".into(), Box::new(move || ungroup(&ui_ungroup, &group_ungroup))),
            ("Close All Terminals".into(), Box::new(move || close_group_terminals(&ui_close, &group_close))),
        ],
    );
}

fn rename_tab(ui: &Rc<RefCell<Ui>>, tab: &Rc<Tab>) {
    let current = tab.title.borrow().clone();
    let Some(title) = prompt(&ui.borrow().window, "Rename tab", &current) else {
        return;
    };
    *tab.title.borrow_mut() = title.trim().to_string();
    refresh_labels(ui);
    refresh_header(ui);
    schedule_save(ui);
}

fn rename_group(ui: &Rc<RefCell<Ui>>, group: &Rc<Group>) {
    let current = group.name.borrow().clone();
    let Some(name) = prompt(&ui.borrow().window, "Rename group", &current) else {
        return;
    };
    set_group_name(ui, group, &name);
}

fn prompt(parent: &impl IsA<gtk::Window>, title: &str, current: &str) -> Option<String> {
    let dialog = gtk::Dialog::new();
    dialog.set_title(title);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button("Rename", gtk::ResponseType::Ok);
    dialog.set_default_response(gtk::ResponseType::Ok);
    let entry = gtk::Entry::new();
    entry.set_text(current);
    entry.set_activates_default(true);
    let area = dialog.content_area();
    area.set_margin_top(12);
    area.set_margin_bottom(12);
    area.set_margin_start(12);
    area.set_margin_end(12);
    area.add(&entry);
    dialog.show_all();
    let response = dialog.run();
    let text = entry.text().to_string();
    dialog.close();
    if response == gtk::ResponseType::Ok && !text.trim().is_empty() {
        Some(text)
    } else {
        None
    }
}

fn refresh_labels(ui: &Rc<RefCell<Ui>>) {
    for tab in all_tabs(ui) {
        let name = display_name(&tab);
        tab.label.set_text(&name);
        tab.label.set_tooltip_text(Some(&tooltip(&tab)));
    }
    for group in ui.borrow().groups.clone() {
        group.label.set_text(&display_group_name(&group.name.borrow(), group.named));
    }
}

fn refresh_header(ui: &Rc<RefCell<Ui>>) {
    let Some(tab) = current_tab(ui) else {
        let state = ui.borrow();
        state.header.set_subtitle(None);
        state.header.set_title(Some("Multiterm"));
        state.window.set_title("Multiterm");
        state.open_button.set_sensitive(false);
        return;
    };
    let path = display_path(&tab.cwd.borrow());
    let name = display_name(&tab);
    let state = ui.borrow();
    state.header.set_subtitle(Some(&path));
    state.header.set_title(Some(&name));
    state.window.set_title(&format!("{name} — Multiterm"));
    state.open_button.set_sensitive(std::path::Path::new(&*tab.cwd.borrow()).is_dir());
}

fn schedule_save(ui: &Rc<RefCell<Ui>>) {
    if ui.borrow().quitting || ui.borrow().save_scheduled {
        return;
    }
    ui.borrow_mut().save_scheduled = true;
    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(SAVE_MS), move || {
        ui.borrow_mut().save_scheduled = false;
        if !ui.borrow().quitting {
            save_now(&ui);
        }
        ControlFlow::Break
    });
}

fn save_now(ui: &Rc<RefCell<Ui>>) {
    let state = ui.borrow();
    let (mut width, mut height) = state.window.size();
    if width < 200 || height < 120 {
        width = 1000;
        height = 640;
    }
    let current = current_group_id(&state);
    let mut active_group = 0;
    let mut groups = Vec::new();
    for (index, group) in state.groups.iter().enumerate() {
        if current.as_deref() == Some(group.id.as_str()) {
            active_group = index as i64;
        }
        let tabs = ordered_tabs(group)
            .into_iter()
            .map(|tab| TabRecord { id: tab.id.clone(), cwd: tab.cwd.borrow().clone(), title: tab.title.borrow().clone() })
            .collect();
        groups.push(GroupRecord {
            id: group.id.clone(),
            name: if group.named { group.name.borrow().clone() } else { String::new() },
            named: group.named,
            active: group.notebook.current_page().unwrap_or(0) as i64,
            tabs,
        });
    }
    drop(state);
    session::save_session(&Session { groups, active_group, width: width as i64, height: height as i64 });
}

fn current_group_id(state: &Ui) -> Option<String> {
    let index = state.groups_notebook.current_page()?;
    let page = state.groups_notebook.nth_page(Some(index))?;
    state.groups.iter().find(|group| same_widget(&group.notebook, &page)).map(|group| group.id.clone())
}

fn poll_directories(ui: &Rc<RefCell<Ui>>) -> ControlFlow {
    if ui.borrow().quitting {
        return ControlFlow::Break;
    }
    let mut changed = false;
    for tab in all_tabs(ui) {
        if let Some(cwd) = pid_cwd(tab.pid.get())
            && cwd != *tab.cwd.borrow()
        {
            *tab.cwd.borrow_mut() = cwd;
            changed = true;
        }
    }
    if changed {
        refresh_labels(ui);
        refresh_header(ui);
        schedule_save(ui);
    }
    ControlFlow::Continue
}

fn watch_settings(ui: &Rc<RefCell<Ui>>) {
    if let Some(settings) = style::terminal_settings() {
        let ui_watch = ui.clone();
        settings.connect_changed(None, move |_, _| reload_appearance(&ui_watch));
    }
    let profiles = gio::Settings::new("org.gnome.Terminal.ProfilesList");
    let ui_default = ui.clone();
    profiles.connect_changed(Some("default"), move |_, _| {
        bind_profile(&ui_default, style::gnome_profile_settings());
        reload_appearance(&ui_default);
    });
    bind_profile(ui, style::gnome_profile_settings());
}

fn bind_profile(ui: &Rc<RefCell<Ui>>, settings: Option<gio::Settings>) {
    if let Some(previous) = ui.borrow().profile_settings.clone()
        && let Some(handler) = ui.borrow_mut().profile_handler.take()
    {
        previous.disconnect(handler);
    }
    ui.borrow_mut().profile_settings = settings.clone();
    if let Some(settings) = settings {
        let watched = ui.clone();
        let handler = settings.connect_changed(None, move |_, _| reload_appearance(&watched));
        ui.borrow_mut().profile_handler = Some(handler);
    }
}

fn reload_appearance(ui: &Rc<RefCell<Ui>>) {
    style::apply_terminal_theme_preference();
    ui.borrow_mut().look = style::load_terminal_look();
    let look = ui.borrow().look.clone();
    for tab in all_tabs(ui) {
        style::apply_terminal_look(&tab.terminal, &look);
        apply_scrollbar(ui, &tab);
    }
}

fn show_group(ui: &Rc<RefCell<Ui>>, group: &Group) {
    let notebook = ui.borrow().groups_notebook.clone();
    if let Some(index) = notebook.page_num(&group.notebook) {
        notebook.set_current_page(Some(index));
    }
}

fn move_group(ui: &Rc<RefCell<Ui>>, group: &Rc<Group>, position: usize) {
    let (notebook, position) = {
        let mut state = ui.borrow_mut();
        let Some(index) = state.groups.iter().position(|item| item.id == group.id) else {
            return;
        };
        let group = state.groups.remove(index);
        let position = position.min(state.groups.len());
        state.groups.insert(position, group.clone());
        (state.groups_notebook.clone(), position)
    };
    notebook.reorder_child(&group.notebook, Some(position as u32));
}

fn remove_group(ui: &Rc<RefCell<Ui>>, group: &Rc<Group>) {
    if !group.named {
        return;
    }
    let (loose, notebook, page) = {
        let mut state = ui.borrow_mut();
        let Some(index) = state.groups.iter().position(|item| item.id == group.id) else {
            return;
        };
        state.groups.remove(index);
        let page = state.groups_notebook.page_num(&group.notebook);
        let loose = state.groups.iter().find(|item| !item.named).cloned();
        (loose, state.groups_notebook.clone(), page)
    };
    if let Some(page) = page {
        notebook.remove_page(Some(page));
    }
    let loose = loose.unwrap_or_else(|| add_group(ui, "", None, false));
    show_group(ui, &loose);
    sync_group_tabs(ui);
    schedule_save(ui);
}

fn sync_group_tabs(ui: &Rc<RefCell<Ui>>) {
    let show = ui.borrow().groups.iter().any(|group| group.named);
    ui.borrow().groups_notebook.set_show_tabs(show);
}

fn loose_group(ui: &Rc<RefCell<Ui>>) -> Rc<Group> {
    if let Some(group) = ui.borrow().groups.iter().find(|group| !group.named).cloned() {
        return group;
    }
    let group = add_group(ui, "", None, false);
    move_group(ui, &group, 0);
    group
}

fn active_group(ui: &Rc<RefCell<Ui>>) -> Rc<Group> {
    current_group(ui).unwrap_or_else(|| {
        let group = add_group(ui, "", None, false);
        show_group(ui, &group);
        group
    })
}

fn current_group(ui: &Rc<RefCell<Ui>>) -> Option<Rc<Group>> {
    let state = ui.borrow();
    let index = state.groups_notebook.current_page()?;
    let page = state.groups_notebook.nth_page(Some(index))?;
    state.groups.iter().find(|group| same_widget(&group.notebook, &page)).cloned().or_else(|| state.groups.first().cloned())
}

fn current_tab(ui: &Rc<RefCell<Ui>>) -> Option<Rc<Tab>> {
    let group = current_group(ui)?;
    let index = group.notebook.current_page()?;
    let page = group.notebook.nth_page(Some(index))?;
    let found = group.tabs.borrow().iter().find(|tab| same_widget(&tab.page, &page)).cloned();
    found
}

fn all_tabs(ui: &Rc<RefCell<Ui>>) -> Vec<Rc<Tab>> {
    ui.borrow().groups.iter().flat_map(|group| ordered_tabs(group)).collect()
}

fn ordered_tabs(group: &Group) -> Vec<Rc<Tab>> {
    let mut found = Vec::new();
    for index in 0..group.notebook.n_pages() {
        let Some(page) = group.notebook.nth_page(Some(index)) else {
            continue;
        };
        if let Some(tab) = group.tabs.borrow().iter().find(|tab| same_widget(&tab.page, &page)).cloned() {
            found.push(tab);
        }
    }
    found
}

fn tab_by_id(ui: &Rc<RefCell<Ui>>, id: &str) -> Option<Rc<Tab>> {
    all_tabs(ui).into_iter().find(|tab| tab.id == id)
}

fn next_group_name(ui: &Rc<RefCell<Ui>>) -> String {
    let existing: HashSet<String> = ui
        .borrow()
        .groups
        .iter()
        .filter(|group| group.named)
        .map(|group| group.name.borrow().clone())
        .collect();
    let mut index = 1;
    loop {
        let name = format!("Group {index}");
        if !existing.contains(&name) {
            return name;
        }
        index += 1;
    }
}

fn insert_position(_ui: &Rc<RefCell<Ui>>, group: &Group, focus: bool) -> u32 {
    let count = group.notebook.n_pages();
    if !focus || new_tab_position() == "last" {
        return count;
    }
    group.notebook.current_page().map(|page| page + 1).unwrap_or(count)
}

fn new_tab_position() -> String {
    style::terminal_settings()
        .map(|settings| settings.string("new-tab-position").to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "last".to_string())
}

fn display_name(tab: &Tab) -> String {
    let title = tab.title.borrow().trim().to_string();
    if title.is_empty() { display_path(&tab.cwd.borrow()) } else { title }
}

fn tooltip(tab: &Tab) -> String {
    let path = display_path(&tab.cwd.borrow());
    let title = tab.title.borrow().trim().to_string();
    if !title.is_empty() && title != path { format!("{title}\n{path}") } else { path }
}

fn display_group_name(name: &str, named: bool) -> String {
    if named && !name.is_empty() { name.to_string() } else { "Ungrouped".to_string() }
}

fn same_widget(left: &impl IsA<gtk::Widget>, right: &impl IsA<gtk::Widget>) -> bool {
    left.as_ref() == right.as_ref()
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(Some(icon), gtk::IconSize::Button);
    button.set_tooltip_text(Some(tooltip));
    button
}

fn start_bench(ui: &Rc<RefCell<Ui>>) {
    let wanted = std::env::var("MULTITERM_BENCH_TABS").ok().and_then(|value| value.parse().ok()).unwrap_or(1);
    println!("BENCH ready");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    while all_tabs(ui).len() < wanted {
        new_tab_action(ui);
    }
    let started = Instant::now();
    glib::timeout_add_local(Duration::from_millis(50), {
        let ui = ui.clone();
        move || bench_shells(&ui, started, wanted)
    });
}

fn bench_shells(ui: &Rc<RefCell<Ui>>, started: Instant, wanted: usize) -> ControlFlow {
    let tabs = all_tabs(ui);
    let ready = tabs.iter().take(wanted).all(|tab| session::ready_path(&tab.history).exists());
    if !ready {
        if started.elapsed() > Duration::from_secs(20) {
            println!(r#"BENCH metrics {{"shells_ms":-1,"echo_ms":-1,"tabs":0}}"#);
            let _ = std::io::Write::flush(&mut std::io::stdout());
            return ControlFlow::Break;
        }
        return ControlFlow::Continue;
    }
    let shells_ms = started.elapsed().as_millis();
    for tab in tabs.iter().take(wanted) {
        tab.terminal.feed_child(b"printf 'multiterm-echo-marker\\n'\n");
    }
    let echo_started = Instant::now();
    glib::timeout_add_local(Duration::from_millis(20), {
        let ui = ui.clone();
        move || bench_echo(&ui, shells_ms, echo_started, wanted)
    });
    ControlFlow::Break
}

fn bench_echo(ui: &Rc<RefCell<Ui>>, shells_ms: u128, started: Instant, wanted: usize) -> ControlFlow {
    let tabs = all_tabs(ui);
    let echoed = tabs.iter().take(wanted).all(|tab| {
        tab.terminal
            .text_format(vte::Format::Text)
            .map(|text| text.contains("multiterm-echo-marker"))
            .unwrap_or(false)
    });
    if !echoed && started.elapsed() < Duration::from_secs(10) {
        return ControlFlow::Continue;
    }
    let echo_ms = if echoed { started.elapsed().as_millis() } else { u128::MAX };
    let echo_ms = if echo_ms == u128::MAX { -1 } else { echo_ms as i128 };
    println!(r#"BENCH metrics {{"shells_ms":{shells_ms},"echo_ms":{echo_ms},"tabs":{}}}"#, tabs.len().min(wanted));
    let _ = std::io::Write::flush(&mut std::io::stdout());
    ControlFlow::Break
}
