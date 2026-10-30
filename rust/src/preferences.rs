//! Preferences window. The pages and the keys are the GNOME Terminal ones.

use std::cell::RefCell;
use std::rc::Rc;

use gio::prelude::*;
use gtk::glib::{self, ControlFlow, Propagation};
use gtk::glib::translate::IntoGlib;
use gtk::pango;
use gtk::prelude::*;

use crate::paths::new_id;
use crate::style::rgba_to_hex;

const PROFILE_SCHEMA: &str = "org.gnome.Terminal.Legacy.Profile";

const COLOR_SCHEMES: &[(&str, &str, &str)] = &[
    ("Black on light yellow", "#000000", "#ffffdd"),
    ("Black on white", "#000000", "#ffffff"),
    ("Gray on black", "#aaaaaa", "#000000"),
    ("Green on black", "#00ff00", "#000000"),
    ("White on black", "#ffffff", "#000000"),
    ("GNOME light", "#171421", "#ffffff"),
    ("GNOME dark", "#d0cfcc", "#171421"),
    ("Tango light", "#2e3436", "#eeeeec"),
    ("Tango dark", "#d3d7cf", "#2e3436"),
    ("Solarized light", "#657b83", "#fdf6e3"),
    ("Solarized dark", "#839496", "#002b36"),
];

const PALETTES: &[(&str, &[&str])] = &[
    (
        "GNOME",
        &[
            "#171421", "#c01c28", "#26a269", "#a2734c", "#12488b", "#a347ba", "#2aa1b3", "#d0cfcc",
            "#5e5c64", "#f66151", "#33d17a", "#e9ad0c", "#2a7bde", "#c061cb", "#33c7de", "#ffffff",
        ],
    ),
    (
        "Tango",
        &[
            "#2e3436", "#cc0000", "#4e9a06", "#c4a000", "#3465a4", "#75507b", "#06989a", "#d3d7cf",
            "#555753", "#ef2929", "#8ae234", "#fce94f", "#729fcf", "#ad7fa8", "#34e2e2", "#eeeeec",
        ],
    ),
    (
        "Linux console",
        &[
            "#000000", "#aa0000", "#00aa00", "#aa5500", "#0000aa", "#aa00aa", "#00aaaa", "#aaaaaa",
            "#555555", "#ff5555", "#55ff55", "#ffff55", "#5555ff", "#ff55ff", "#55ffff", "#ffffff",
        ],
    ),
    (
        "XTerm",
        &[
            "#000000", "#cd0000", "#00cd00", "#cdcd00", "#0000ee", "#cd00cd", "#00cdcd", "#e5e5e5",
            "#7f7f7f", "#ff0000", "#00ff00", "#ffff00", "#5c5cff", "#ff00ff", "#00ffff", "#ffffff",
        ],
    ),
    (
        "Rxvt",
        &[
            "#000000", "#cd0000", "#00cd00", "#cdcd00", "#0000cd", "#cd00cd", "#00cdcd", "#faebd7",
            "#404040", "#ff0000", "#00ff00", "#ffff00", "#0000ff", "#ff00ff", "#00ffff", "#ffffff",
        ],
    ),
    (
        "Solarized",
        &[
            "#073642", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198", "#eee8d5",
            "#002b36", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4", "#93a1a1", "#fdf6e3",
        ],
    ),
];

const ENCODINGS: &[(&str, &str)] = &[
    ("UTF-8", "Unicode"),
    ("BIG5", "Chinese Traditional"),
    ("BIG5-HKSCS", "Chinese Traditional"),
    ("EUC-JP", "Japanese"),
    ("EUC-KR", "Korean"),
    ("EUC-TW", "Chinese Traditional"),
    ("GB18030", "Chinese Simplified"),
    ("GB2312", "Chinese Simplified"),
    ("GBK", "Chinese Simplified"),
    ("SHIFT_JIS", "Japanese"),
    ("UHC", "Korean"),
    ("ISO-8859-1", "Western"),
    ("ISO-8859-15", "Western"),
    ("ISO-8859-2", "Central European"),
    ("ISO-8859-5", "Cyrillic"),
    ("ISO-8859-7", "Greek"),
    ("ISO-8859-9", "Turkish"),
    ("KOI8-R", "Cyrillic"),
    ("WINDOWS-1251", "Cyrillic"),
    ("WINDOWS-1252", "Western"),
];

struct Choice {
    dropdown: gtk::ComboBoxText,
    ids: Vec<String>,
}

impl Choice {
    fn new(options: &[(&str, &str)]) -> Self {
        let dropdown = gtk::ComboBoxText::new();
        for (_, label) in options {
            dropdown.append(None, label);
        }
        if !options.is_empty() {
            dropdown.set_active(Some(0));
        }
        Self {
            dropdown,
            ids: options.iter().map(|(id, _)| (*id).to_string()).collect(),
        }
    }

    fn selected_id(&self) -> String {
        self.ids.get(self.dropdown.active().unwrap_or(0) as usize).cloned().unwrap_or_default()
    }

    fn select_id(&self, id: &str) {
        let index = self.ids.iter().position(|item| item == id).unwrap_or(0);
        self.dropdown.set_active(Some(index as u32));
    }
}

struct Prefs {
    window: gtk::Window,
    loading: RefCell<bool>,
    profiles: gio::Settings,
    terminal: gio::Settings,
    bindings: gio::Settings,
    profile: RefCell<Option<gio::Settings>>,
    profile_ids: RefCell<Vec<String>>,
    palette: RefCell<Vec<String>>,
    capturing: RefCell<Option<(String, gtk::Button)>>,
    global_list: gtk::ListBox,
    profile_list: gtk::ListBox,
    stack: gtk::Stack,
    profile_id_label: gtk::Label,
    custom_font: gtk::CheckButton,
    font: gtk::FontButton,
    columns: gtk::SpinButton,
    rows: gtk::SpinButton,
    cell_width: gtk::SpinButton,
    cell_height: gtk::SpinButton,
    text_blink: Choice,
    cursor_shape: Choice,
    cursor_blink: Choice,
    audible_bell: gtk::CheckButton,
    theme_colors: gtk::CheckButton,
    scheme: Choice,
    foreground: gtk::ColorButton,
    background: gtk::ColorButton,
    bold_enabled: gtk::CheckButton,
    bold: gtk::ColorButton,
    cursor_colors: gtk::CheckButton,
    cursor_foreground: gtk::ColorButton,
    cursor_background: gtk::ColorButton,
    highlight_colors: gtk::CheckButton,
    highlight_foreground: gtk::ColorButton,
    highlight_background: gtk::ColorButton,
    transparent: gtk::CheckButton,
    transparency: gtk::Scale,
    theme_transparency: gtk::CheckButton,
    palette_scheme: Choice,
    palette_buttons: Vec<gtk::ColorButton>,
    bold_is_bright: gtk::CheckButton,
    scrollbar: gtk::CheckButton,
    scroll_on_output: gtk::CheckButton,
    scroll_on_keystroke: gtk::CheckButton,
    scroll_on_insert: gtk::CheckButton,
    limit: gtk::CheckButton,
    scrollback: gtk::SpinButton,
    login_shell: gtk::CheckButton,
    custom_command_enabled: gtk::CheckButton,
    custom_command: gtk::Entry,
    preserve: Choice,
    exit_action: Choice,
    backspace: Choice,
    delete: Choice,
    encoding: Choice,
    cjk: Choice,
    shortcut_list: gtk::ListBox,
}

pub fn open_preferences(parent: &gtk::Window) {
    let Some(built) = Prefs::build(parent) else {
        return;
    };
    let prefs = Rc::new(built);
    prefs.clone().wire();
    let slot = Rc::new(RefCell::new(Some(prefs.clone())));
    prefs.window.connect_delete_event(move |_, _| {
        let slot = slot.clone();
        glib::idle_add_local(move || {
            slot.borrow_mut().take();
            ControlFlow::Break
        });
        Propagation::Proceed
    });
    prefs.window.show_all();
    prefs.window.present();
}

impl Prefs {
    fn build(parent: &gtk::Window) -> Option<Self> {
        let profiles = gio::Settings::new("org.gnome.Terminal.ProfilesList");
        let terminal = gio::Settings::new("org.gnome.Terminal.Legacy.Settings");
        let bindings = terminal.child("keybindings");
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_transient_for(Some(parent));
        window.set_modal(true);
        window.set_title("Preferences");
        window.set_default_size(920, 640);

        let global_list = gtk::ListBox::new();
        global_list.style_context().add_class("navigation-sidebar");
        for label in ["General", "Shortcuts"] {
            let row = gtk::ListBoxRow::new();
            let text = gtk::Label::new(Some(label));
            text.set_xalign(0.0);
            text.set_margin_top(6);
            text.set_margin_bottom(6);
            text.set_margin_start(8);
            text.set_margin_end(8);
            row.add(&text);
            global_list.add(&row);
        }
        let profile_list = gtk::ListBox::new();
        profile_list.style_context().add_class("navigation-sidebar");
        let add = gtk::Button::from_icon_name(Some("list-add-symbolic"), gtk::IconSize::Button);
        add.set_tooltip_text(Some("New profile"));
        let side = gtk::Box::new(gtk::Orientation::Vertical, 6);
        side.set_margin_top(6);
        side.set_margin_bottom(6);
        side.set_margin_start(6);
        side.set_margin_end(6);
        side.add(&gtk::Label::new(Some("Global")));
        side.add(&global_list);
        side.add(&gtk::Label::new(Some("Profiles")));
        side.add(&add);
        let scrolled_profiles = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scrolled_profiles.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scrolled_profiles.set_vexpand(true);
        scrolled_profiles.add(&profile_list);
        side.add(&scrolled_profiles);

        let stack = gtk::Stack::new();
        let general = general_page();
        let shortcuts_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
        shortcuts_box.set_margin_top(12);
        shortcuts_box.set_margin_bottom(12);
        shortcuts_box.set_margin_start(12);
        shortcuts_box.set_margin_end(12);
        let (profile_page, profile_widgets) = profile_page();
        stack.add_named(&general.page, "general");
        stack.add_named(&shortcuts_box, "shortcuts");
        stack.add_named(&profile_page, "profile");
        stack.set_hexpand(true);
        stack.set_vexpand(true);

        let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        root.add(&side);
        root.add(&stack);
        window.add(&root);

        let shortcut_list = gtk::ListBox::new();
        let prefs = Self {
            window,
            loading: RefCell::new(true),
            profiles,
            terminal,
            bindings,
            profile: RefCell::new(None),
            profile_ids: RefCell::new(Vec::new()),
            palette: RefCell::new(vec!["#000000".into(); 16]),
            capturing: RefCell::new(None),
            global_list,
            profile_list,
            stack,
            profile_id_label: profile_widgets.profile_id_label,
            custom_font: profile_widgets.custom_font,
            font: profile_widgets.font,
            columns: profile_widgets.columns,
            rows: profile_widgets.rows,
            cell_width: profile_widgets.cell_width,
            cell_height: profile_widgets.cell_height,
            text_blink: profile_widgets.text_blink,
            cursor_shape: profile_widgets.cursor_shape,
            cursor_blink: profile_widgets.cursor_blink,
            audible_bell: profile_widgets.audible_bell,
            theme_colors: profile_widgets.theme_colors,
            scheme: profile_widgets.scheme,
            foreground: profile_widgets.foreground,
            background: profile_widgets.background,
            bold_enabled: profile_widgets.bold_enabled,
            bold: profile_widgets.bold,
            cursor_colors: profile_widgets.cursor_colors,
            cursor_foreground: profile_widgets.cursor_foreground,
            cursor_background: profile_widgets.cursor_background,
            highlight_colors: profile_widgets.highlight_colors,
            highlight_foreground: profile_widgets.highlight_foreground,
            highlight_background: profile_widgets.highlight_background,
            transparent: profile_widgets.transparent,
            transparency: profile_widgets.transparency,
            theme_transparency: profile_widgets.theme_transparency,
            palette_scheme: profile_widgets.palette_scheme,
            palette_buttons: profile_widgets.palette_buttons,
            bold_is_bright: profile_widgets.bold_is_bright,
            scrollbar: profile_widgets.scrollbar,
            scroll_on_output: profile_widgets.scroll_on_output,
            scroll_on_keystroke: profile_widgets.scroll_on_keystroke,
            scroll_on_insert: profile_widgets.scroll_on_insert,
            limit: profile_widgets.limit,
            scrollback: profile_widgets.scrollback,
            login_shell: profile_widgets.login_shell,
            custom_command_enabled: profile_widgets.custom_command_enabled,
            custom_command: profile_widgets.custom_command,
            preserve: profile_widgets.preserve,
            exit_action: profile_widgets.exit_action,
            backspace: profile_widgets.backspace,
            delete: profile_widgets.delete,
            encoding: profile_widgets.encoding,
            cjk: profile_widgets.cjk,
            shortcut_list,
        };
        prefs.fill_general(&general);
        prefs.fill_shortcuts(&shortcuts_box);
        let _ = add;
        Some(prefs)
    }

    fn wire(self: &Rc<Self>) {
        let prefs = self.clone();
        self.global_list.connect_row_selected(move |_, row| {
            let Some(row) = row else {
                return;
            };
            if *prefs.loading.borrow() {
                return;
            }
            prefs.profile_list.unselect_all();
            let name = if row.index() == 1 { "shortcuts" } else { "general" };
            prefs.stack.set_visible_child_name(name);
        });
        let prefs = self.clone();
        self.profile_list.connect_row_selected(move |_, row| {
            let Some(row) = row else {
                return;
            };
            if *prefs.loading.borrow() {
                return;
            }
            let index = row.index();
            if index < 0 {
                return;
            }
            let id = prefs.profile_ids.borrow().get(index as usize).cloned();
            if let Some(id) = id {
                prefs.global_list.unselect_all();
                prefs.show_profile(&id);
            }
        });
        self.wire_add_button();
        self.wire_profile_controls();
        self.rebuild_profiles();
        *self.loading.borrow_mut() = true;
        self.global_list.select_row(self.global_list.row_at_index(0).as_ref());
        self.stack.set_visible_child_name("general");
        *self.loading.borrow_mut() = false;
    }

    fn wire_add_button(self: &Rc<Self>) {
        let prefs = self.clone();
        visit_buttons(self.window.upcast_ref(), move |button| {
            if button.tooltip_text().as_deref() == Some("New profile") {
                let prefs = prefs.clone();
                button.connect_clicked(move |_| prefs.new_profile());
            }
        });
    }

    fn wire_profile_controls(self: &Rc<Self>) {
        let bind_check = |prefs: &Rc<Self>, button: &gtk::CheckButton, key: &'static str, invert: bool| {
            let prefs = prefs.clone();
            let button = button.clone();
            let reading = button.clone();
            button.connect_toggled(move |_| {
                if *prefs.loading.borrow() {
                    return;
                }
                let value = if invert { !reading.is_active() } else { reading.is_active() };
                prefs.write_bool(key, value);
                prefs.refresh_sensitivity();
            });
        };
        bind_check(self, &self.custom_font, "use-system-font", true);
        bind_check(self, &self.bold_enabled, "bold-color-same-as-fg", true);
        bind_check(self, &self.theme_colors, "use-theme-colors", false);
        bind_check(self, &self.cursor_colors, "cursor-colors-set", false);
        bind_check(self, &self.highlight_colors, "highlight-colors-set", false);
        bind_check(self, &self.transparent, "use-transparent-background", false);
        bind_check(self, &self.theme_transparency, "use-theme-transparency", false);
        bind_check(self, &self.bold_is_bright, "bold-is-bright", false);
        bind_check(self, &self.scroll_on_output, "scroll-on-output", false);
        bind_check(self, &self.scroll_on_keystroke, "scroll-on-keystroke", false);
        bind_check(self, &self.scroll_on_insert, "scroll-on-insert", false);
        bind_check(self, &self.audible_bell, "audible-bell", false);
        bind_check(self, &self.login_shell, "login-shell", false);
        bind_check(self, &self.custom_command_enabled, "use-custom-command", false);
        bind_check(self, &self.limit, "scrollback-unlimited", true);

        let prefs = self.clone();
        self.scrollbar.connect_toggled(move |button| {
            if *prefs.loading.borrow() {
                return;
            }
            let policy = if button.is_active() { "always" } else { "never" };
            prefs.write_string("scrollbar-policy", policy);
        });
        let prefs = self.clone();
        self.font.connect_font_set(move |button| {
            if *prefs.loading.borrow() {
                return;
            }
            let desc = button.property::<pango::FontDescription>("font-desc");
            prefs.write_string("font", desc.to_str().as_str());
        });
        let bind_color = |prefs: &Rc<Self>, button: &gtk::ColorButton, key: &'static str| {
            let prefs = prefs.clone();
            button.connect_color_set(move |button| {
                if *prefs.loading.borrow() {
                    return;
                }
                prefs.write_string(key, &rgba_to_hex(&button.rgba()));
            });
        };
        bind_color(self, &self.foreground, "foreground-color");
        bind_color(self, &self.background, "background-color");
        bind_color(self, &self.bold, "bold-color");
        bind_color(self, &self.cursor_foreground, "cursor-foreground-color");
        bind_color(self, &self.cursor_background, "cursor-background-color");
        bind_color(self, &self.highlight_foreground, "highlight-foreground-color");
        bind_color(self, &self.highlight_background, "highlight-background-color");
        for (index, button) in self.palette_buttons.iter().enumerate() {
            let prefs = self.clone();
            button.connect_color_set(move |button| {
                if *prefs.loading.borrow() {
                    return;
                }
                let mut colors = prefs.palette.borrow().clone();
                if index >= colors.len() {
                    return;
                }
                colors[index] = rgba_to_hex(&button.rgba());
                *prefs.palette.borrow_mut() = colors.clone();
                prefs.write_strv("palette", &colors);
                *prefs.loading.borrow_mut() = true;
                prefs.palette_scheme.select_id("custom");
                *prefs.loading.borrow_mut() = false;
            });
        }
        let bind_choice = |prefs: &Rc<Self>, choice: &Choice, key: &'static str| {
            let prefs = prefs.clone();
            let ids = choice.ids.clone();
            choice.dropdown.connect_changed(move |dropdown| {
                if *prefs.loading.borrow() {
                    return;
                }
                let Some(id) = ids.get(dropdown.active().unwrap_or(0) as usize) else {
                    return;
                };
                prefs.write_string(key, id);
            });
        };
        bind_choice(self, &self.text_blink, "text-blink-mode");
        bind_choice(self, &self.cursor_shape, "cursor-shape");
        bind_choice(self, &self.cursor_blink, "cursor-blink-mode");
        bind_choice(self, &self.preserve, "preserve-working-directory");
        bind_choice(self, &self.exit_action, "exit-action");
        bind_choice(self, &self.backspace, "backspace-binding");
        bind_choice(self, &self.delete, "delete-binding");
        bind_choice(self, &self.cjk, "cjk-utf8-ambiguous-width");
        let prefs = self.clone();
        let encoding_ids = self.encoding.ids.clone();
        self.encoding.dropdown.connect_changed(move |dropdown| {
            if *prefs.loading.borrow() {
                return;
            }
            if let Some(id) = encoding_ids.get(dropdown.active().unwrap_or(0) as usize) {
                prefs.write_string("encoding", id);
            }
        });
        let prefs = self.clone();
        let scheme_ids = self.scheme.ids.clone();
        self.scheme.dropdown.connect_changed(move |dropdown| {
            if *prefs.loading.borrow() {
                return;
            }
            let Some(name) = scheme_ids.get(dropdown.active().unwrap_or(0) as usize) else {
                return;
            };
            for (scheme, foreground, background) in COLOR_SCHEMES {
                if scheme == name {
                    prefs.write_string("foreground-color", foreground);
                    prefs.write_string("background-color", background);
                    prefs.write_bool("use-theme-colors", false);
                    *prefs.loading.borrow_mut() = true;
                    prefs.theme_colors.set_active(false);
                    prefs.foreground.set_rgba(&crate::style::rgba(foreground));
                    prefs.background.set_rgba(&crate::style::rgba(background));
                    prefs.refresh_sensitivity();
                    *prefs.loading.borrow_mut() = false;
                }
            }
        });
        let prefs = self.clone();
        let palette_ids = self.palette_scheme.ids.clone();
        self.palette_scheme.dropdown.connect_changed(move |dropdown| {
            if *prefs.loading.borrow() {
                return;
            }
            let Some(name) = palette_ids.get(dropdown.active().unwrap_or(0) as usize) else {
                return;
            };
            for (scheme, colors) in PALETTES {
                if scheme == name {
                    let owned: Vec<String> = colors.iter().map(|item| (*item).to_string()).collect();
                    *prefs.palette.borrow_mut() = owned.clone();
                    prefs.write_strv("palette", &owned);
                    *prefs.loading.borrow_mut() = true;
                    prefs.paint_palette();
                    *prefs.loading.borrow_mut() = false;
                }
            }
        });
        let bind_spin = |prefs: &Rc<Self>, spin: &gtk::SpinButton, key: &'static str| {
            let prefs = prefs.clone();
            spin.connect_value_changed(move |spin| {
                if *prefs.loading.borrow() {
                    return;
                }
                prefs.write_int(key, spin.value() as i32);
            });
        };
        bind_spin(self, &self.columns, "default-size-columns");
        bind_spin(self, &self.rows, "default-size-rows");
        bind_spin(self, &self.scrollback, "scrollback-lines");
        let bind_double = |prefs: &Rc<Self>, spin: &gtk::SpinButton, key: &'static str| {
            let prefs = prefs.clone();
            spin.connect_value_changed(move |spin| {
                if *prefs.loading.borrow() {
                    return;
                }
                prefs.write_double(key, spin.value());
            });
        };
        bind_double(self, &self.cell_width, "cell-width-scale");
        bind_double(self, &self.cell_height, "cell-height-scale");
        let prefs = self.clone();
        self.transparency.connect_value_changed(move |scale| {
            if *prefs.loading.borrow() {
                return;
            }
            prefs.write_int("background-transparency-percent", scale.value() as i32);
        });
        let prefs = self.clone();
        self.custom_command.connect_changed(move |entry| {
            if *prefs.loading.borrow() {
                return;
            }
            prefs.write_string("custom-command", entry.text().as_str());
        });
        self.wire_resets();
        self.install_key_capture();
    }

    fn wire_resets(self: &Rc<Self>) {
        // Reset buttons are the ones labelled Reset / Reset Compatibility. Found by label.
        let Some(page) = self.stack.child_by_name("profile") else {
            return;
        };
        let prefs = self.clone();
        visit_buttons(&page, move |button| {
            let label = button.label().unwrap_or_default();
            let prefs = prefs.clone();
            if label.as_str() == "Reset" {
                let which = reset_kind(button);
                button.connect_clicked(move |_| {
                    if which == 0 {
                        prefs.write_int("default-size-columns", 80);
                        prefs.write_int("default-size-rows", 24);
                        *prefs.loading.borrow_mut() = true;
                        prefs.columns.set_value(80.0);
                        prefs.rows.set_value(24.0);
                        *prefs.loading.borrow_mut() = false;
                    } else {
                        prefs.write_double("cell-width-scale", 1.0);
                        prefs.write_double("cell-height-scale", 1.0);
                        *prefs.loading.borrow_mut() = true;
                        prefs.cell_width.set_value(1.0);
                        prefs.cell_height.set_value(1.0);
                        *prefs.loading.borrow_mut() = false;
                    }
                });
            } else if label.as_str() == "Reset Compatibility Options to Defaults" {
                button.connect_clicked(move |_| {
                    prefs.write_string("backspace-binding", "ascii-delete");
                    prefs.write_string("delete-binding", "delete-sequence");
                    prefs.write_string("encoding", "UTF-8");
                    prefs.write_string("cjk-utf8-ambiguous-width", "narrow");
                    *prefs.loading.borrow_mut() = true;
                    prefs.backspace.select_id("ascii-delete");
                    prefs.delete.select_id("delete-sequence");
                    prefs.encoding.select_id("UTF-8");
                    prefs.cjk.select_id("narrow");
                    *prefs.loading.borrow_mut() = false;
                });
            }
        });
    }

    fn fill_general(&self, page: &GeneralPage) {
        bind_settings_check(&self.terminal, &page.mnemonics, "mnemonics-enabled");
        bind_settings_check(&self.terminal, &page.menu_accel, "menu-accelerator-enabled");
        bind_settings_check(&self.terminal, &page.always_check, "always-check-default-terminal");
        select_setting(&self.terminal, &page.theme, "theme-variant", "system");
        select_setting(&self.terminal, &page.terminal_mode, "new-terminal-mode", "window");
        select_setting(&self.terminal, &page.tab_position, "new-tab-position", "last");
        let terminal = self.terminal.clone();
        let ids = page.theme.ids.clone();
        page.theme.dropdown.connect_changed(move |dropdown| {
            if let Some(id) = ids.get(dropdown.active().unwrap_or(0) as usize) {
                write_settings_string(&terminal, "theme-variant", id);
            }
        });
        let terminal = self.terminal.clone();
        let ids = page.terminal_mode.ids.clone();
        page.terminal_mode.dropdown.connect_changed(move |dropdown| {
            if let Some(id) = ids.get(dropdown.active().unwrap_or(0) as usize) {
                write_settings_string(&terminal, "new-terminal-mode", id);
            }
        });
        let terminal = self.terminal.clone();
        let ids = page.tab_position.ids.clone();
        page.tab_position.dropdown.connect_changed(move |dropdown| {
            if let Some(id) = ids.get(dropdown.active().unwrap_or(0) as usize) {
                write_settings_string(&terminal, "new-tab-position", id);
            }
        });
        let window = self.window.clone();
        page.make_default.connect_clicked(move |_| make_default_terminal(&window));
    }

    fn fill_shortcuts(&self, box_: &gtk::Box) {
        let enabled = gtk::CheckButton::with_label("Enable shortcuts");
        enabled.set_active(self.terminal.boolean("shortcuts-enabled"));
        let terminal = self.terminal.clone();
        enabled.connect_toggled(move |button| {
            write_settings_bool(&terminal, "shortcuts-enabled", button.is_active());
        });
        box_.add(&enabled);
        let list = self.shortcut_list.clone();
        let scrolled = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scrolled.set_vexpand(true);
        scrolled.add(&list);
        box_.add(&scrolled);
        let Some(schema) = self.bindings.settings_schema() else {
            return;
        };
        let mut keys: Vec<String> = schema.list_keys().iter().map(|key| key.to_string()).collect();
        keys.sort_by_key(|key| shortcut_label(&schema, key).to_lowercase());
        for key in keys {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row.set_margin_top(4);
            row.set_margin_bottom(4);
            row.set_margin_start(8);
            row.set_margin_end(8);
            let label = gtk::Label::new(Some(&shortcut_label(&schema, &key)));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            let current = self.bindings.string(&key).to_string();
            let button = gtk::Button::with_label(&accelerator_label(&current));
            let clear = gtk::Button::with_label("Disabled");
            let bindings = self.bindings.clone();
            let key_name = key.clone();
            let button_for_clear = button.clone();
            clear.connect_clicked(move |_| {
                write_settings_string(&bindings, &key_name, "disabled");
                button_for_clear.set_label("Disabled");
            });
            row.add(&label);
            row.add(&button);
            row.add(&clear);
            list.add(&row);
        }
    }

    fn install_key_capture(self: &Rc<Self>) {
        let list = self.shortcut_list.clone();
        let Some(schema) = self.bindings.settings_schema() else {
            return;
        };
        let mut keys: Vec<String> = schema.list_keys().iter().map(|key| key.to_string()).collect();
        keys.sort_by_key(|key| shortcut_label(&schema, key).to_lowercase());
        for (widget, key) in container_children(&list).into_iter().zip(keys) {
            let nested = container_children(&widget);
            let container = nested
                .iter()
                .find_map(|child| child.clone().downcast::<gtk::Box>().ok())
                .or_else(|| widget.clone().downcast::<gtk::Box>().ok());
            let Some(container) = container else {
                continue;
            };
            let Some(change) = container_children(&container)
                .into_iter()
                .find_map(|item| item.downcast::<gtk::Button>().ok())
            else {
                continue;
            };
            let prefs = self.clone();
            change.connect_clicked(move |button| {
                button.set_label("Press a key…");
                *prefs.capturing.borrow_mut() = Some((key.clone(), button.clone()));
            });
        }
        let prefs = self.clone();
        self.window.connect_key_press_event(move |_, event| {
            let Some((name, button)) = prefs.capturing.borrow_mut().take() else {
                return Propagation::Proceed;
            };
            let key = event.keyval();
            if key == gtk::gdk::keys::constants::Escape {
                let current = prefs.bindings.string(&name);
                button.set_label(&accelerator_label(current.as_str()));
                return Propagation::Stop;
            }
            let stored = if key == gtk::gdk::keys::constants::BackSpace {
                "disabled".to_string()
            } else {
                let mods = event.state()
                    & (gtk::gdk::ModifierType::CONTROL_MASK
                        | gtk::gdk::ModifierType::SHIFT_MASK
                        | gtk::gdk::ModifierType::MOD1_MASK
                        | gtk::gdk::ModifierType::SUPER_MASK);
                gtk::accelerator_name(key.into_glib(), mods)
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "disabled".to_string())
            };
            write_settings_string(&prefs.bindings, &name, &stored);
            button.set_label(&accelerator_label(&stored));
            Propagation::Stop
        });
    }

    fn rebuild_profiles(&self) {
        *self.loading.borrow_mut() = true;
        for child in container_children(&self.profile_list) {
            self.profile_list.remove(&child);
        }
        let listed: Vec<String> = self.profiles.strv("list").iter().map(|item| item.to_string()).collect();
        let default = self.profiles.string("default").to_string();
        *self.profile_ids.borrow_mut() = listed.clone();
        for id in &listed {
            self.profile_list.add(&self.profile_row(id, id == &default));
        }
        *self.loading.borrow_mut() = false;
    }

    fn profile_row(&self, id: &str, default: bool) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();
        let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        box_.set_margin_top(4);
        box_.set_margin_bottom(4);
        box_.set_margin_start(8);
        box_.set_margin_end(4);
        let mark = gtk::Image::from_icon_name(Some("emblem-ok-symbolic"), gtk::IconSize::Button);
        mark.set_visible(default);
        let name = gtk::Label::new(Some(&profile_name(id)));
        name.set_xalign(0.0);
        name.set_hexpand(true);
        name.set_ellipsize(pango::EllipsizeMode::End);
        let menu = gtk::Button::from_icon_name(Some("pan-down-symbolic"), gtk::IconSize::Button);
        menu.style_context().add_class("flat");
        let id_owned = id.to_string();
        let window = self.window.clone();
        let profiles = self.profiles.clone();
        let list = self.profile_list.clone();
        let ids = self.profile_ids.clone();
        let loading = self.loading.clone();
        menu.connect_clicked(move |button| {
            popup_profile_menu(&window, button, &profiles, &list, &ids, &loading, &id_owned);
        });
        box_.add(&mark);
        box_.add(&name);
        box_.add(&menu);
        row.add(&box_);
        row
    }

    fn show_profile(&self, id: &str) {
        let Some(settings) = open_profile(id) else {
            return;
        };
        *self.loading.borrow_mut() = true;
        *self.profile.borrow_mut() = Some(settings.clone());
        self.profile_id_label.set_text(id);
        self.custom_font.set_active(!settings.boolean("use-system-font"));
        self.font.set_property("font-desc", pango::FontDescription::from_string(&settings.string("font")));
        self.columns.set_value(settings.int("default-size-columns") as f64);
        self.rows.set_value(settings.int("default-size-rows") as f64);
        self.cell_width.set_value(settings.double("cell-width-scale"));
        self.cell_height.set_value(settings.double("cell-height-scale"));
        self.text_blink.select_id(settings.string("text-blink-mode").as_str());
        self.cursor_shape.select_id(settings.string("cursor-shape").as_str());
        self.cursor_blink.select_id(settings.string("cursor-blink-mode").as_str());
        self.audible_bell.set_active(settings.boolean("audible-bell"));
        self.theme_colors.set_active(settings.boolean("use-theme-colors"));
        self.foreground.set_rgba(&crate::style::rgba(settings.string("foreground-color").as_str()));
        self.background.set_rgba(&crate::style::rgba(settings.string("background-color").as_str()));
        self.bold_enabled.set_active(!settings.boolean("bold-color-same-as-fg"));
        self.bold.set_rgba(&crate::style::rgba(settings.string("bold-color").as_str()));
        self.cursor_colors.set_active(settings.boolean("cursor-colors-set"));
        self.cursor_foreground.set_rgba(&crate::style::rgba(settings.string("cursor-foreground-color").as_str()));
        self.cursor_background.set_rgba(&crate::style::rgba(settings.string("cursor-background-color").as_str()));
        self.highlight_colors.set_active(settings.boolean("highlight-colors-set"));
        self.highlight_foreground
            .set_rgba(&crate::style::rgba(settings.string("highlight-foreground-color").as_str()));
        self.highlight_background
            .set_rgba(&crate::style::rgba(settings.string("highlight-background-color").as_str()));
        self.transparent.set_active(settings.boolean("use-transparent-background"));
        self.theme_transparency.set_active(settings.boolean("use-theme-transparency"));
        self.transparency.set_value(settings.int("background-transparency-percent") as f64);
        self.bold_is_bright.set_active(settings.boolean("bold-is-bright"));
        let stored: Vec<String> = settings.strv("palette").iter().map(|item| item.to_string()).collect();
        if stored.len() >= 16 {
            *self.palette.borrow_mut() = stored[..16].to_vec();
        }
        self.paint_palette();
        self.match_schemes();
        self.scrollbar.set_active(settings.string("scrollbar-policy").as_str() != "never");
        self.scroll_on_output.set_active(settings.boolean("scroll-on-output"));
        self.scroll_on_keystroke.set_active(settings.boolean("scroll-on-keystroke"));
        self.scroll_on_insert.set_active(settings.boolean("scroll-on-insert"));
        self.limit.set_active(!settings.boolean("scrollback-unlimited"));
        self.scrollback.set_value(settings.int("scrollback-lines") as f64);
        self.login_shell.set_active(settings.boolean("login-shell"));
        self.custom_command_enabled.set_active(settings.boolean("use-custom-command"));
        self.custom_command.set_text(settings.string("custom-command").as_str());
        self.preserve.select_id(settings.string("preserve-working-directory").as_str());
        self.exit_action.select_id(settings.string("exit-action").as_str());
        self.backspace.select_id(settings.string("backspace-binding").as_str());
        self.delete.select_id(settings.string("delete-binding").as_str());
        self.encoding.select_id(settings.string("encoding").as_str());
        self.cjk.select_id(settings.string("cjk-utf8-ambiguous-width").as_str());
        self.refresh_sensitivity();
        self.stack.set_visible_child_name("profile");
        self.profile_list.show_all();
        *self.loading.borrow_mut() = false;
    }

    fn paint_palette(&self) {
        let colors = self.palette.borrow().clone();
        for (index, button) in self.palette_buttons.iter().enumerate() {
            let spec = colors.get(index).map(String::as_str).unwrap_or("#000000");
            button.set_rgba(&crate::style::rgba(spec));
        }
    }

    fn match_schemes(&self) {
        let Some(settings) = self.profile.borrow().clone() else {
            return;
        };
        let foreground = settings.string("foreground-color").to_string();
        let background = settings.string("background-color").to_string();
        let mut scheme = "custom";
        for (name, fg, bg) in COLOR_SCHEMES {
            if foreground.eq_ignore_ascii_case(fg) && background.eq_ignore_ascii_case(bg) {
                scheme = name;
                break;
            }
        }
        self.scheme.select_id(scheme);
        let colors = self.palette.borrow().clone();
        let mut palette = "custom";
        for (name, values) in PALETTES {
            if colors.iter().zip(values.iter()).all(|(left, right)| left.eq_ignore_ascii_case(right)) {
                palette = name;
                break;
            }
        }
        self.palette_scheme.select_id(palette);
    }

    fn refresh_sensitivity(&self) {
        let theme = self.theme_colors.is_active();
        self.scheme.dropdown.set_sensitive(!theme);
        self.foreground.set_sensitive(!theme);
        self.background.set_sensitive(!theme);
        self.bold.set_sensitive(self.bold_enabled.is_active());
        self.cursor_foreground.set_sensitive(self.cursor_colors.is_active());
        self.cursor_background.set_sensitive(self.cursor_colors.is_active());
        self.highlight_foreground.set_sensitive(self.highlight_colors.is_active());
        self.highlight_background.set_sensitive(self.highlight_colors.is_active());
        let transparent = self.transparent.is_active() && !self.theme_transparency.is_active();
        self.transparency.set_sensitive(transparent);
        self.font.set_sensitive(self.custom_font.is_active());
        self.scrollback.set_sensitive(self.limit.is_active());
        self.custom_command.set_sensitive(self.custom_command_enabled.is_active());
    }

    fn new_profile(&self) {
        let Some(name) = ask_name(&self.window, "New Profile", "Unnamed") else {
            return;
        };
        self.create_profile(&name, None);
    }

    fn create_profile(&self, name: &str, source_id: Option<&str>) {
        let id = uuid_string();
        let Some(settings) = open_profile(&id) else {
            return;
        };
        if let Some(source_id) = source_id {
            if let Some(source) = open_profile(source_id) {
                if let Some(schema) = source.settings_schema() {
                    for key in schema.list_keys() {
                        let _ = settings.set_value(key.as_str(), &source.value(key.as_str()));
                    }
                }
            }
        }
        let _ = settings.set_string("visible-name", name);
        let mut listed: Vec<String> = self.profiles.strv("list").iter().map(|item| item.to_string()).collect();
        listed.push(id.clone());
        let refs: Vec<&str> = listed.iter().map(String::as_str).collect();
        let _ = self.profiles.set_strv("list", refs);
        self.rebuild_profiles();
        self.select_profile(&id);
    }

    fn select_profile(&self, id: &str) {
        let index = self.profile_ids.borrow().iter().position(|item| item == id);
        let Some(index) = index else {
            return;
        };
        *self.loading.borrow_mut() = true;
        self.global_list.unselect_all();
        if let Some(row) = self.profile_list.row_at_index(index as i32) {
            self.profile_list.select_row(Some(&row));
        }
        *self.loading.borrow_mut() = false;
        self.show_profile(id);
    }

    fn write_bool(&self, key: &str, value: bool) {
        if let Some(settings) = self.profile.borrow().clone() {
            write_settings_bool(&settings, key, value);
        }
    }

    fn write_string(&self, key: &str, value: &str) {
        if let Some(settings) = self.profile.borrow().clone() {
            write_settings_string(&settings, key, value);
        }
    }

    fn write_int(&self, key: &str, value: i32) {
        if let Some(settings) = self.profile.borrow().clone()
            && settings.int(key) != value
        {
            let _ = settings.set_int(key, value);
        }
    }

    fn write_double(&self, key: &str, value: f64) {
        if let Some(settings) = self.profile.borrow().clone()
            && (settings.double(key) - value).abs() > 0.0001
        {
            let _ = settings.set_double(key, value);
        }
    }

    fn write_strv(&self, key: &str, values: &[String]) {
        if let Some(settings) = self.profile.borrow().clone() {
            let current: Vec<String> = settings.strv(key).iter().map(|item| item.to_string()).collect();
            if current == values {
                return;
            }
            let refs: Vec<&str> = values.iter().map(String::as_str).collect();
            let _ = settings.set_strv(key, refs);
        }
    }
}

struct GeneralPage {
    page: gtk::ScrolledWindow,
    mnemonics: gtk::CheckButton,
    menu_accel: gtk::CheckButton,
    always_check: gtk::CheckButton,
    theme: Choice,
    terminal_mode: Choice,
    tab_position: Choice,
    make_default: gtk::Button,
}

struct ProfileWidgets {
    profile_id_label: gtk::Label,
    custom_font: gtk::CheckButton,
    font: gtk::FontButton,
    columns: gtk::SpinButton,
    rows: gtk::SpinButton,
    cell_width: gtk::SpinButton,
    cell_height: gtk::SpinButton,
    text_blink: Choice,
    cursor_shape: Choice,
    cursor_blink: Choice,
    audible_bell: gtk::CheckButton,
    theme_colors: gtk::CheckButton,
    scheme: Choice,
    foreground: gtk::ColorButton,
    background: gtk::ColorButton,
    bold_enabled: gtk::CheckButton,
    bold: gtk::ColorButton,
    cursor_colors: gtk::CheckButton,
    cursor_foreground: gtk::ColorButton,
    cursor_background: gtk::ColorButton,
    highlight_colors: gtk::CheckButton,
    highlight_foreground: gtk::ColorButton,
    highlight_background: gtk::ColorButton,
    transparent: gtk::CheckButton,
    transparency: gtk::Scale,
    theme_transparency: gtk::CheckButton,
    palette_scheme: Choice,
    palette_buttons: Vec<gtk::ColorButton>,
    bold_is_bright: gtk::CheckButton,
    scrollbar: gtk::CheckButton,
    scroll_on_output: gtk::CheckButton,
    scroll_on_keystroke: gtk::CheckButton,
    scroll_on_insert: gtk::CheckButton,
    limit: gtk::CheckButton,
    scrollback: gtk::SpinButton,
    login_shell: gtk::CheckButton,
    custom_command_enabled: gtk::CheckButton,
    custom_command: gtk::Entry,
    preserve: Choice,
    exit_action: Choice,
    backspace: Choice,
    delete: Choice,
    encoding: Choice,
    cjk: Choice,
}

fn general_page() -> GeneralPage {
    let (box_, page) = scrolled_page();
    let mnemonics = check("Enable mnemonics (such as Alt+F to open the File menu)");
    let menu_accel = check("Enable the menu accelerator key (F10 by default)");
    box_.add(&mnemonics);
    box_.add(&menu_accel);
    let theme = Choice::new(&[
        ("system", "Follow system style"),
        ("light", "Light style"),
        ("dark", "Dark style"),
    ]);
    let terminal_mode = Choice::new(&[("window", "Window"), ("tab", "Tab")]);
    let tab_position = Choice::new(&[("last", "Last"), ("next", "Next")]);
    labeled(&box_, "Theme variant:", &theme.dropdown);
    labeled(&box_, "Open new terminals in:", &terminal_mode.dropdown);
    labeled(&box_, "New tab position:", &tab_position.dropdown);
    let always_check = check("Always check if default terminal");
    box_.add(&always_check);
    let make_default = gtk::Button::with_label("Set as default terminal");
    make_default.set_halign(gtk::Align::Start);
    box_.add(&make_default);
    GeneralPage { page, mnemonics, menu_accel, always_check, theme, terminal_mode, tab_position, make_default }
}

fn profile_page() -> (gtk::Notebook, ProfileWidgets) {
    let notebook = gtk::Notebook::new();
    let text = text_page();
    let colors = color_page();
    let scrolling = scrolling_page();
    let command = command_page();
    let compatibility = compatibility_page();
    notebook.append_page(&text.0, Some(&gtk::Label::new(Some("Text"))));
    notebook.append_page(&colors.0, Some(&gtk::Label::new(Some("Colors"))));
    notebook.append_page(&scrolling.0, Some(&gtk::Label::new(Some("Scrolling"))));
    notebook.append_page(&command.0, Some(&gtk::Label::new(Some("Command"))));
    notebook.append_page(&compatibility.0, Some(&gtk::Label::new(Some("Compatibility"))));
    let widgets = ProfileWidgets {
        profile_id_label: text.1,
        custom_font: text.2,
        font: text.3,
        columns: text.4,
        rows: text.5,
        cell_width: text.6,
        cell_height: text.7,
        text_blink: text.8,
        cursor_shape: text.9,
        cursor_blink: text.10,
        audible_bell: text.11,
        theme_colors: colors.1,
        scheme: colors.2,
        foreground: colors.3,
        background: colors.4,
        bold_enabled: colors.5,
        bold: colors.6,
        cursor_colors: colors.7,
        cursor_foreground: colors.8,
        cursor_background: colors.9,
        highlight_colors: colors.10,
        highlight_foreground: colors.11,
        highlight_background: colors.12,
        transparent: colors.13,
        transparency: colors.14,
        theme_transparency: colors.15,
        palette_scheme: colors.16,
        palette_buttons: colors.17,
        bold_is_bright: colors.18,
        scrollbar: scrolling.1,
        scroll_on_output: scrolling.2,
        scroll_on_keystroke: scrolling.3,
        scroll_on_insert: scrolling.4,
        limit: scrolling.5,
        scrollback: scrolling.6,
        login_shell: command.1,
        custom_command_enabled: command.2,
        custom_command: command.3,
        preserve: command.4,
        exit_action: command.5,
        backspace: compatibility.1,
        delete: compatibility.2,
        encoding: compatibility.3,
        cjk: compatibility.4,
    };
    (notebook, widgets)
}

#[allow(clippy::type_complexity)]
fn text_page() -> (
    gtk::ScrolledWindow,
    gtk::Label,
    gtk::CheckButton,
    gtk::FontButton,
    gtk::SpinButton,
    gtk::SpinButton,
    gtk::SpinButton,
    gtk::SpinButton,
    Choice,
    Choice,
    Choice,
    gtk::CheckButton,
) {
    let (box_, page) = scrolled_page();
    box_.add(&heading("Text Appearance"));
    let columns = spin_int(16.0, 511.0);
    let rows = spin_int(4.0, 511.0);
    let size = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    size.add(&columns);
    size.add(&gtk::Label::new(Some("columns")));
    size.add(&rows);
    size.add(&gtk::Label::new(Some("rows")));
    let reset_size = gtk::Button::with_label("Reset");
    reset_size.set_widget_name("reset-size");
    size.add(&reset_size);
    labeled(&box_, "Initial terminal size:", &size);
    let custom_font = check("Custom font:");
    let font = gtk::FontButton::new();
    let fonts = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    fonts.add(&custom_font);
    fonts.add(&font);
    box_.add(&fonts);
    let cell_width = spin_float();
    let cell_height = spin_float();
    let spacing = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    spacing.add(&cell_width);
    spacing.add(&gtk::Label::new(Some("width")));
    spacing.add(&cell_height);
    spacing.add(&gtk::Label::new(Some("height")));
    let reset_cells = gtk::Button::with_label("Reset");
    reset_cells.set_widget_name("reset-cells");
    spacing.add(&reset_cells);
    labeled(&box_, "Cell spacing:", &spacing);
    let text_blink = Choice::new(&[
        ("never", "Never"),
        ("focused", "When focused"),
        ("unfocused", "When unfocused"),
        ("always", "Always"),
    ]);
    labeled(&box_, "Allow blinking text:", &text_blink.dropdown);
    box_.add(&heading("Cursor"));
    let cursor_shape = Choice::new(&[("block", "Block"), ("ibeam", "I-Beam"), ("underline", "Underline")]);
    let cursor_blink = Choice::new(&[("system", "Default"), ("on", "Enabled"), ("off", "Disabled")]);
    labeled(&box_, "Cursor shape:", &cursor_shape.dropdown);
    labeled(&box_, "Cursor blinking:", &cursor_blink.dropdown);
    box_.add(&heading("Sound"));
    let audible = check("Terminal bell");
    box_.add(&audible);
    let profile_id = gtk::Label::new(None);
    profile_id.set_xalign(0.0);
    profile_id.set_selectable(true);
    profile_id.set_margin_top(18);
    let identity = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    identity.add(&gtk::Label::new(Some("Profile ID:")));
    identity.add(&profile_id);
    box_.add(&identity);
    (
        page, profile_id, custom_font, font, columns, rows, cell_width, cell_height, text_blink, cursor_shape,
        cursor_blink, audible,
    )
}

#[allow(clippy::type_complexity)]
fn color_page() -> (
    gtk::ScrolledWindow,
    gtk::CheckButton,
    Choice,
    gtk::ColorButton,
    gtk::ColorButton,
    gtk::CheckButton,
    gtk::ColorButton,
    gtk::CheckButton,
    gtk::ColorButton,
    gtk::ColorButton,
    gtk::CheckButton,
    gtk::ColorButton,
    gtk::ColorButton,
    gtk::CheckButton,
    gtk::Scale,
    gtk::CheckButton,
    Choice,
    Vec<gtk::ColorButton>,
    gtk::CheckButton,
) {
    let (box_, page) = scrolled_page();
    box_.add(&heading("Text and Background Color"));
    let theme_colors = check("Use colors from system theme");
    box_.add(&theme_colors);
    let mut scheme_options = vec![("custom", "Custom")];
    for (name, _, _) in COLOR_SCHEMES {
        scheme_options.push((name, name));
    }
    let scheme = Choice::new(&scheme_options);
    labeled(&box_, "Built-in schemes:", &scheme.dropdown);
    let foreground = color_button();
    let background = color_button();
    let pair = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    pair.add(&foreground);
    pair.add(&background);
    labeled(&box_, "Default color:", &pair);
    let bold_enabled = check("Bold color:");
    let bold = color_button();
    let bold_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bold_row.add(&bold_enabled);
    bold_row.add(&bold);
    box_.add(&bold_row);
    let cursor_colors = check("Cursor color:");
    let cursor_foreground = color_button();
    let cursor_background = color_button();
    let cursor_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    cursor_row.add(&cursor_colors);
    cursor_row.add(&cursor_foreground);
    cursor_row.add(&cursor_background);
    box_.add(&cursor_row);
    let highlight_colors = check("Highlight color:");
    let highlight_foreground = color_button();
    let highlight_background = color_button();
    let highlight_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    highlight_row.add(&highlight_colors);
    highlight_row.add(&highlight_foreground);
    highlight_row.add(&highlight_background);
    box_.add(&highlight_row);
    let transparent = check("Use transparent background");
    let transparency = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    transparency.set_draw_value(false);
    transparency.set_hexpand(true);
    transparency.set_size_request(240, -1);
    let transparent_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    transparent_row.add(&transparent);
    transparent_row.add(&gtk::Label::new(Some("none")));
    transparent_row.add(&transparency);
    transparent_row.add(&gtk::Label::new(Some("full")));
    box_.add(&transparent_row);
    let theme_transparency = check("Use transparency from system theme");
    box_.add(&theme_transparency);
    box_.add(&heading("Palette"));
    let mut palette_options = vec![("custom", "Custom")];
    for (name, _) in PALETTES {
        palette_options.push((name, name));
    }
    let palette_scheme = Choice::new(&palette_options);
    labeled(&box_, "Built-in schemes:", &palette_scheme.dropdown);
    let grid = gtk::Grid::new();
    grid.set_column_spacing(4);
    grid.set_row_spacing(4);
    let mut palette_buttons = Vec::new();
    for index in 0..16 {
        let button = color_button();
        grid.attach(&button, (index % 8) as i32, (index / 8) as i32, 1, 1);
        palette_buttons.push(button);
    }
    labeled(&box_, "Color palette:", &grid);
    let bold_is_bright = check("Show bold text in bright colors");
    box_.add(&bold_is_bright);
    (
        page,
        theme_colors,
        scheme,
        foreground,
        background,
        bold_enabled,
        bold,
        cursor_colors,
        cursor_foreground,
        cursor_background,
        highlight_colors,
        highlight_foreground,
        highlight_background,
        transparent,
        transparency,
        theme_transparency,
        palette_scheme,
        palette_buttons,
        bold_is_bright,
    )
}

fn scrolling_page() -> (
    gtk::ScrolledWindow,
    gtk::CheckButton,
    gtk::CheckButton,
    gtk::CheckButton,
    gtk::CheckButton,
    gtk::CheckButton,
    gtk::SpinButton,
) {
    let (box_, page) = scrolled_page();
    let scrollbar = check("Show scrollbar");
    let scroll_on_output = check("Scroll on output");
    let scroll_on_keystroke = check("Scroll on keystroke");
    let scroll_on_insert = check("Scroll on paste");
    let limit = check("Limit scrollback to:");
    let scrollback = spin_int(1.0, 10_000_000.0);
    let limited = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    limited.add(&limit);
    limited.add(&scrollback);
    limited.add(&gtk::Label::new(Some("lines")));
    box_.add(&scrollbar);
    box_.add(&scroll_on_output);
    box_.add(&scroll_on_keystroke);
    box_.add(&scroll_on_insert);
    box_.add(&limited);
    (page, scrollbar, scroll_on_output, scroll_on_keystroke, scroll_on_insert, limit, scrollback)
}

fn command_page() -> (
    gtk::ScrolledWindow,
    gtk::CheckButton,
    gtk::CheckButton,
    gtk::Entry,
    Choice,
    Choice,
) {
    let (box_, page) = scrolled_page();
    let login = check("Run command as a login shell");
    let custom = check("Run a custom command instead of my shell");
    let entry = gtk::Entry::new();
    entry.set_hexpand(true);
    let preserve = Choice::new(&[("never", "Never"), ("safe", "Shell only"), ("always", "Always")]);
    let exit_action = Choice::new(&[
        ("close", "Exit the terminal"),
        ("restart", "Restart the command"),
        ("hold", "Hold the terminal open"),
    ]);
    box_.add(&login);
    box_.add(&custom);
    labeled(&box_, "Custom command:", &entry);
    labeled(&box_, "Preserve working directory:", &preserve.dropdown);
    labeled(&box_, "When command exits:", &exit_action.dropdown);
    (page, login, custom, entry, preserve, exit_action)
}

fn compatibility_page() -> (gtk::ScrolledWindow, Choice, Choice, Choice, Choice) {
    let (box_, page) = scrolled_page();
    let erase = &[
        ("auto", "Automatic"),
        ("ascii-backspace", "Control-H"),
        ("ascii-delete", "ASCII DEL"),
        ("delete-sequence", "Escape sequence"),
        ("tty", "TTY Erase"),
    ];
    let backspace = Choice::new(erase);
    let delete = Choice::new(erase);
    let encoding_options: Vec<(String, String)> = ENCODINGS
        .iter()
        .map(|(charset, language)| ((*charset).to_string(), format!("{language} — {charset}")))
        .collect();
    let encoding_refs: Vec<(&str, &str)> =
        encoding_options.iter().map(|(id, label)| (id.as_str(), label.as_str())).collect();
    let encoding = Choice::new(&encoding_refs);
    let cjk = Choice::new(&[("narrow", "Narrow"), ("wide", "Wide")]);
    labeled(&box_, "Backspace key generates:", &backspace.dropdown);
    labeled(&box_, "Delete key generates:", &delete.dropdown);
    labeled(&box_, "Encoding:", &encoding.dropdown);
    labeled(&box_, "Ambiguous-width characters:", &cjk.dropdown);
    let reset = gtk::Button::with_label("Reset Compatibility Options to Defaults");
    reset.set_halign(gtk::Align::Start);
    box_.add(&reset);
    let _ = encoding_options;
    (page, backspace, delete, encoding, cjk)
}

fn scrolled_page() -> (gtk::Box, gtk::ScrolledWindow) {
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 8);
    box_.set_margin_top(12);
    box_.set_margin_bottom(12);
    box_.set_margin_start(12);
    box_.set_margin_end(12);
    let scrolled = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scrolled.add(&box_);
    (box_, scrolled)
}

fn heading(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.style_context().add_class("heading");
    label.set_margin_top(8);
    label
}

fn check(label: &str) -> gtk::CheckButton {
    gtk::CheckButton::with_label(label)
}

fn labeled(parent: &gtk::Box, caption: &str, widget: &impl IsA<gtk::Widget>) {
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let label = gtk::Label::new(Some(caption));
    label.set_xalign(0.0);
    line.add(&label);
    line.add(widget);
    parent.add(&line);
}

fn spin_int(lower: f64, upper: f64) -> gtk::SpinButton {
    gtk::SpinButton::with_range(lower, upper, 1.0)
}

fn spin_float() -> gtk::SpinButton {
    let spin = gtk::SpinButton::with_range(0.5, 2.0, 0.1);
    spin.set_digits(2);
    spin
}

fn color_button() -> gtk::ColorButton {
    gtk::ColorButton::new()
}

fn bind_settings_check(settings: &gio::Settings, button: &gtk::CheckButton, key: &'static str) {
    button.set_active(settings.boolean(key));
    let settings = settings.clone();
    button.connect_toggled(move |button| write_settings_bool(&settings, key, button.is_active()));
}

fn select_setting(settings: &gio::Settings, choice: &Choice, key: &str, fallback: &str) {
    let value = settings.string(key);
    let value = if value.is_empty() { fallback } else { value.as_str() };
    choice.select_id(value);
}

fn write_settings_bool(settings: &gio::Settings, key: &str, value: bool) {
    if settings.boolean(key) != value {
        let _ = settings.set_boolean(key, value);
    }
}

fn write_settings_string(settings: &gio::Settings, key: &str, value: &str) {
    if settings.string(key).as_str() != value {
        let _ = settings.set_string(key, value);
    }
}

fn open_profile(id: &str) -> Option<gio::Settings> {
    Some(gio::Settings::with_path(PROFILE_SCHEMA, &format!("/org/gnome/terminal/legacy/profiles:/{id}/")))
}

fn profile_name(id: &str) -> String {
    open_profile(id)
        .map(|settings| settings.string("visible-name").to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Unnamed".to_string())
}

fn shortcut_label(schema: &gio::SettingsSchema, key: &str) -> String {
    schema.key(key).summary().map(|text| text.to_string()).unwrap_or_else(|| key.to_string())
}

fn accelerator_label(accel: &str) -> String {
    if accel.is_empty() || accel == "disabled" {
        return "Disabled".to_string();
    }
    let (key, mods) = gtk::accelerator_parse(accel);
    if key == 0 {
        return accel.to_string();
    }
    gtk::accelerator_get_label(key, mods)
        .map(|label| label.to_string())
        .unwrap_or_else(|| accel.to_string())
}

fn uuid_string() -> String {
    let id = new_id();
    format!("{}-{}-{}-{}-{}", &id[0..8], &id[8..12], &id[12..16], &id[16..20], &id[20..32])
}

fn ask_name(parent: &gtk::Window, title: &str, current: &str) -> Option<String> {
    let dialog = gtk::Window::new(gtk::WindowType::Toplevel);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.set_title(title);
    dialog.set_default_size(360, 120);
    let entry = gtk::Entry::new();
    entry.set_text(current);
    let ok = gtk::Button::with_label("OK");
    ok.style_context().add_class("suggested-action");
    let cancel = gtk::Button::with_label("Cancel");
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.set_halign(gtk::Align::End);
    buttons.add(&cancel);
    buttons.add(&ok);
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 8);
    box_.set_margin_top(12);
    box_.set_margin_bottom(12);
    box_.set_margin_start(12);
    box_.set_margin_end(12);
    box_.add(&entry);
    box_.add(&buttons);
    dialog.add(&box_);
    let result = Rc::new(RefCell::new(None));
    let loop_ = glib::MainLoop::new(None, false);
    let result_ok = result.clone();
    let entry_ok = entry.clone();
    let loop_ok = loop_.clone();
    let dialog_ok = dialog.clone();
    ok.connect_clicked(move |_| {
        let name = entry_ok.text().trim().to_string();
        if !name.is_empty() {
            *result_ok.borrow_mut() = Some(name);
        }
        dialog_ok.close();
        loop_ok.quit();
    });
    let loop_cancel = loop_.clone();
    let dialog_cancel = dialog.clone();
    cancel.connect_clicked(move |_| {
        dialog_cancel.close();
        loop_cancel.quit();
    });
    let loop_close = loop_.clone();
    dialog.connect_delete_event(move |_, _| {
        loop_close.quit();
        Propagation::Proceed
    });
    dialog.present();
    loop_.run();
    result.borrow().clone()
}

fn make_default_terminal(parent: &gtk::Window) {
    let desktop = "/usr/share/applications/org.gnome.Terminal.desktop";
    if !std::path::Path::new(desktop).exists() {
        show_error(parent, "GNOME Terminal is not available.");
        return;
    }
    let status = std::process::Command::new("gio")
        .args(["mime", "x-scheme-handler/terminal", "org.gnome.Terminal.desktop"])
        .status();
    if !matches!(status, Ok(code) if code.success()) {
        show_error(parent, "Could not set the default terminal.");
    }
}

fn show_error(parent: &gtk::Window, message: &str) {
    let dialog = gtk::MessageDialog::new(
        Some(parent),
        gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
        gtk::MessageType::Error,
        gtk::ButtonsType::Close,
        message,
    );
    dialog.run();
    dialog.close();
}

fn popup_profile_menu(
    window: &gtk::Window,
    anchor: &gtk::Button,
    profiles: &gio::Settings,
    list: &gtk::ListBox,
    ids: &RefCell<Vec<String>>,
    loading: &RefCell<bool>,
    id: &str,
) {
    let popover = gtk::Popover::new(Some(anchor));
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let id = id.to_string();
    let actions: Vec<(&str, Box<dyn Fn()>)> = vec![
        (
            "Rename",
            Box::new({
                let window = window.clone();
                let profiles = profiles.clone();
                let list = list.clone();
                let ids = ids.clone();
                let loading = loading.clone();
                let id = id.clone();
                move || {
                    let Some(name) = ask_name(&window, "Rename Profile", &profile_name(&id)) else {
                        return;
                    };
                    if let Some(settings) = open_profile(&id) {
                        let _ = settings.set_string("visible-name", &name);
                    }
                    rebuild_from(profiles.clone(), list.clone(), ids.clone(), loading.clone());
                }
            }),
        ),
        (
            "Clone",
            Box::new({
                let window = window.clone();
                let profiles = profiles.clone();
                let list = list.clone();
                let ids = ids.clone();
                let loading = loading.clone();
                let id = id.clone();
                move || {
                    let suggested = format!("{} (Copy)", profile_name(&id));
                    let Some(name) = ask_name(&window, "Clone Profile", &suggested) else {
                        return;
                    };
                    let new_id = uuid_string();
                    let Some(settings) = open_profile(&new_id) else {
                        return;
                    };
                    if let Some(source) = open_profile(&id) {
                        if let Some(schema) = source.settings_schema() {
                            for key in schema.list_keys() {
                                let _ = settings.set_value(key.as_str(), &source.value(key.as_str()));
                            }
                        }
                    }
                    let _ = settings.set_string("visible-name", &name);
                    let mut listed: Vec<String> =
                        profiles.strv("list").iter().map(|item| item.to_string()).collect();
                    listed.push(new_id);
                    let refs: Vec<&str> = listed.iter().map(String::as_str).collect();
                    let _ = profiles.set_strv("list", refs);
                    rebuild_from(profiles.clone(), list.clone(), ids.clone(), loading.clone());
                }
            }),
        ),
        (
            "Set as default",
            Box::new({
                let profiles = profiles.clone();
                let list = list.clone();
                let ids = ids.clone();
                let loading = loading.clone();
                let id = id.clone();
                move || {
                    let _ = profiles.set_string("default", &id);
                    rebuild_from(profiles.clone(), list.clone(), ids.clone(), loading.clone());
                }
            }),
        ),
        (
            "Delete",
            Box::new({
                let profiles = profiles.clone();
                let list = list.clone();
                let ids = ids.clone();
                let loading = loading.clone();
                let id = id.clone();
                move || {
                    let mut listed: Vec<String> =
                        profiles.strv("list").iter().map(|item| item.to_string()).collect();
                    if listed.len() <= 1 || !listed.iter().any(|item| item == &id) {
                        return;
                    }
                    listed.retain(|item| item != &id);
                    if profiles.string("default").as_str() == id {
                        let _ = profiles.set_string("default", &listed[0]);
                    }
                    let refs: Vec<&str> = listed.iter().map(String::as_str).collect();
                    let _ = profiles.set_strv("list", refs);
                    rebuild_from(profiles.clone(), list.clone(), ids.clone(), loading.clone());
                }
            }),
        ),
    ];
    for (label, action) in actions {
        let button = gtk::Button::with_label(label);
        button.style_context().add_class("flat");
        let popover = popover.clone();
        button.connect_clicked(move |_| {
            action();
            popover.popdown();
        });
        box_.add(&button);
    }
    popover.add(&box_);
    box_.show_all();
    popover.popup();
}

fn rebuild_from(profiles: gio::Settings, list: gtk::ListBox, ids: RefCell<Vec<String>>, loading: RefCell<bool>) {
    *loading.borrow_mut() = true;
    for child in container_children(&list) {
        list.remove(&child);
    }
    let listed: Vec<String> = profiles.strv("list").iter().map(|item| item.to_string()).collect();
    let default = profiles.string("default").to_string();
    *ids.borrow_mut() = listed.clone();
    for id in &listed {
        let row = gtk::ListBoxRow::new();
        let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        box_.set_margin_start(8);
        box_.set_margin_top(4);
        box_.set_margin_bottom(4);
        let mark = gtk::Image::from_icon_name(Some("emblem-ok-symbolic"), gtk::IconSize::Button);
        mark.set_visible(id == &default);
        let name = gtk::Label::new(Some(&profile_name(id)));
        name.set_hexpand(true);
        name.set_xalign(0.0);
        box_.add(&mark);
        box_.add(&name);
        row.add(&box_);
        list.add(&row);
    }
    list.show_all();
    *loading.borrow_mut() = false;
}

fn container_children(widget: &impl IsA<gtk::Widget>) -> Vec<gtk::Widget> {
    let mut found = Vec::new();
    if let Some(container) = widget.as_ref().downcast_ref::<gtk::Container>() {
        container.foreach(|child| found.push(child.clone()));
    }
    found
}

fn visit_buttons(widget: &gtk::Widget, mut visit: impl FnMut(&gtk::Button)) {
    fn walk(widget: &gtk::Widget, visit: &mut dyn FnMut(&gtk::Button)) {
        if let Some(button) = widget.downcast_ref::<gtk::Button>() {
            visit(button);
        }
        for child in container_children(widget) {
            walk(&child, visit);
        }
    }
    walk(widget, &mut visit);
}

fn reset_kind(button: &gtk::Button) -> i32 {
    if button.widget_name().as_str() == "reset-cells" { 1 } else { 0 }
}
