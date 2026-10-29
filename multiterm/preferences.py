"""Preferences window with the same pages and the same settings as GNOME Terminal.

Global pages edit ``org.gnome.Terminal.Legacy.Settings``. Profile pages edit
``org.gnome.Terminal.Legacy.Profile``. Nothing is stored in a private copy.
"""

from __future__ import annotations

import uuid

import gi

gi.require_version("Gdk", "3.0")
gi.require_version("Gtk", "3.0")

from gi.repository import Gdk, Gio, GLib, Gtk, Pango

from multiterm.style import gnome_profile_settings, rgba_to_hex, terminal_settings

PROFILE_SCHEMA = "org.gnome.Terminal.Legacy.Profile"
PROFILE_KEYS = (
    "use-system-font",
    "font",
    "default-size-columns",
    "default-size-rows",
    "cell-width-scale",
    "cell-height-scale",
    "text-blink-mode",
    "cursor-shape",
    "cursor-blink-mode",
    "audible-bell",
    "use-theme-colors",
    "foreground-color",
    "background-color",
    "bold-color-same-as-fg",
    "bold-color",
    "cursor-colors-set",
    "cursor-foreground-color",
    "cursor-background-color",
    "highlight-colors-set",
    "highlight-foreground-color",
    "highlight-background-color",
    "palette",
    "use-transparent-background",
    "use-theme-transparency",
    "background-transparency-percent",
    "bold-is-bright",
    "scrollbar-policy",
    "scroll-on-output",
    "scroll-on-keystroke",
    "scroll-on-insert",
    "scrollback-unlimited",
    "scrollback-lines",
    "login-shell",
    "use-custom-command",
    "custom-command",
    "preserve-working-directory",
    "exit-action",
    "backspace-binding",
    "delete-binding",
    "encoding",
    "cjk-utf8-ambiguous-width",
    "visible-name",
)
SETTINGS_KEYS = (
    "mnemonics-enabled",
    "menu-accelerator-enabled",
    "theme-variant",
    "new-terminal-mode",
    "new-tab-position",
    "always-check-default-terminal",
    "shortcuts-enabled",
)

_THEME = (
    ("system", "Follow system style"),
    ("light", "Light style"),
    ("dark", "Dark style"),
)
_TERMINAL_MODE = (("window", "Window"), ("tab", "Tab"))
_TAB_POSITION = (("last", "Last"), ("next", "Next"))
_CURSOR_SHAPE = (("block", "Block"), ("ibeam", "I-Beam"), ("underline", "Underline"))
_CURSOR_BLINK = (("system", "Default"), ("on", "Enabled"), ("off", "Disabled"))
_TEXT_BLINK = (
    ("never", "Never"),
    ("focused", "When focused"),
    ("unfocused", "When unfocused"),
    ("always", "Always"),
)
_EXIT = (
    ("close", "Exit the terminal"),
    ("restart", "Restart the command"),
    ("hold", "Hold the terminal open"),
)
_PRESERVE = (("never", "Never"), ("safe", "Shell only"), ("always", "Always"))
_ERASE = (
    ("auto", "Automatic"),
    ("ascii-backspace", "Control-H"),
    ("ascii-delete", "ASCII DEL"),
    ("delete-sequence", "Escape sequence"),
    ("tty", "TTY Erase"),
)
_CJK = (("narrow", "Narrow"), ("wide", "Wide"))

# Foreground/background presets from GNOME Terminal's profile editor.
_COLOR_SCHEMES = (
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
)
_PALETTES = (
    (
        "GNOME",
        (
            "#171421",
            "#c01c28",
            "#26a269",
            "#a2734c",
            "#12488b",
            "#a347ba",
            "#2aa1b3",
            "#d0cfcc",
            "#5e5c64",
            "#f66151",
            "#33d17a",
            "#e9ad0c",
            "#2a7bde",
            "#c061cb",
            "#33c7de",
            "#ffffff",
        ),
    ),
    (
        "Tango",
        (
            "#2e3436",
            "#cc0000",
            "#4e9a06",
            "#c4a000",
            "#3465a4",
            "#75507b",
            "#06989a",
            "#d3d7cf",
            "#555753",
            "#ef2929",
            "#8ae234",
            "#fce94f",
            "#729fcf",
            "#ad7fa8",
            "#34e2e2",
            "#eeeeec",
        ),
    ),
    (
        "Linux console",
        (
            "#000000",
            "#aa0000",
            "#00aa00",
            "#aa5500",
            "#0000aa",
            "#aa00aa",
            "#00aaaa",
            "#aaaaaa",
            "#555555",
            "#ff5555",
            "#55ff55",
            "#ffff55",
            "#5555ff",
            "#ff55ff",
            "#55ffff",
            "#ffffff",
        ),
    ),
    (
        "XTerm",
        (
            "#000000",
            "#cd0000",
            "#00cd00",
            "#cdcd00",
            "#0000ee",
            "#cd00cd",
            "#00cdcd",
            "#e5e5e5",
            "#7f7f7f",
            "#ff0000",
            "#00ff00",
            "#ffff00",
            "#5c5cff",
            "#ff00ff",
            "#00ffff",
            "#ffffff",
        ),
    ),
    (
        "Rxvt",
        (
            "#000000",
            "#cd0000",
            "#00cd00",
            "#cdcd00",
            "#0000cd",
            "#cd00cd",
            "#00cdcd",
            "#faebd7",
            "#404040",
            "#ff0000",
            "#00ff00",
            "#ffff00",
            "#0000ff",
            "#ff00ff",
            "#00ffff",
            "#ffffff",
        ),
    ),
    (
        "Solarized",
        (
            "#073642",
            "#dc322f",
            "#859900",
            "#b58900",
            "#268bd2",
            "#d33682",
            "#2aa198",
            "#eee8d5",
            "#002b36",
            "#cb4b16",
            "#586e75",
            "#657b83",
            "#839496",
            "#6c71c4",
            "#93a1a1",
            "#fdf6e3",
        ),
    ),
)
_ENCODINGS = (
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
)


def profile_path(profile_id: str) -> str:
    return f"/org/gnome/terminal/legacy/profiles:/{profile_id}/"


def open_profile(profile_id: str):
    try:
        return Gio.Settings.new_with_path(PROFILE_SCHEMA, profile_path(profile_id))
    except GLib.Error:
        return None


class PreferencesDialog(Gtk.Dialog):
    """Sidebar and pages laid out like GNOME Terminal's preferences."""

    def __init__(self, parent: Gtk.Window) -> None:
        super().__init__(transient_for=parent, modal=False, title="Preferences")
        self.set_default_size(880, 640)
        self.set_destroy_with_parent(True)
        self.add_button("Close", Gtk.ResponseType.CLOSE)
        self.connect("response", lambda dialog, _response: dialog.destroy())
        self.terminal = terminal_settings()
        self.profiles = _profiles_list()
        self.profile = gnome_profile_settings()
        self._profile_id = ""
        self._loading = True
        self._palette: list[str] = []
        self._profile_rows: dict[str, Gtk.ListBoxRow] = {}
        body = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        body.pack_start(self._sidebar(), False, False, 0)
        body.pack_start(Gtk.Separator(orientation=Gtk.Orientation.VERTICAL), False, False, 0)
        self._stack = Gtk.Stack()
        self._stack.add_named(self._general_page(), "general")
        self._stack.add_named(self._shortcuts_page(), "shortcuts")
        self._stack.add_named(self._profile_page(), "profile")
        body.pack_start(self._stack, True, True, 0)
        self.get_content_area().add(body)
        self._select_global("general")
        self._loading = False

    def _sidebar(self) -> Gtk.Widget:
        side = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        side.set_size_request(200, -1)
        side.set_margin_top(12)
        side.set_margin_bottom(12)
        side.set_margin_start(12)
        side.set_margin_end(6)
        side.pack_start(self._heading("Global"), False, False, 0)
        self._global_list = Gtk.ListBox()
        self._global_list.set_selection_mode(Gtk.SelectionMode.SINGLE)
        for name, title in (("general", "General"), ("shortcuts", "Shortcuts")):
            row = Gtk.ListBoxRow()
            row.page_name = name
            label = Gtk.Label(label=title, xalign=0)
            label.set_margin_top(6)
            label.set_margin_bottom(6)
            label.set_margin_start(8)
            label.set_margin_end(8)
            row.add(label)
            self._global_list.add(row)
        self._global_list.select_row(self._global_list.get_row_at_index(0))
        self._global_list.connect("row-selected", self._on_global_selected)
        side.pack_start(self._global_list, False, False, 0)
        header = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        header.pack_start(self._heading("Profiles"), True, True, 0)
        add = Gtk.Button.new_from_icon_name("list-add-symbolic", Gtk.IconSize.BUTTON)
        add.set_relief(Gtk.ReliefStyle.NONE)
        add.set_tooltip_text("New profile")
        add.connect("clicked", lambda *_args: self._new_profile())
        header.pack_end(add, False, False, 0)
        side.pack_start(header, False, False, 6)
        self._profile_list = Gtk.ListBox()
        self._profile_list.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self._profile_list.connect("row-selected", self._on_profile_selected)
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scrolled.set_vexpand(True)
        scrolled.add(self._profile_list)
        side.pack_start(scrolled, True, True, 0)
        self._rebuild_profiles()
        return side

    def _general_page(self) -> Gtk.Widget:
        grid = self._grid()
        row = 0
        row = self._span(
            grid,
            row,
            self._check(
                self.terminal,
                "mnemonics-enabled",
                "Enable mnemonics (such as Alt+F to open the File menu)",
            ),
        )
        row = self._span(
            grid,
            row,
            self._check(
                self.terminal,
                "menu-accelerator-enabled",
                "Enable the menu accelerator key (F10 by default)",
            ),
        )
        row = self._row(
            grid,
            row,
            "Theme variant:",
            self._combo(self.terminal, "theme-variant", _THEME, "system"),
        )
        row = self._row(
            grid,
            row,
            "Open new terminals in:",
            self._combo(self.terminal, "new-terminal-mode", _TERMINAL_MODE, "window"),
        )
        row = self._row(
            grid,
            row,
            "New tab position:",
            self._combo(self.terminal, "new-tab-position", _TAB_POSITION, "last"),
        )
        row = self._span(
            grid,
            row,
            self._check(
                self.terminal,
                "always-check-default-terminal",
                "Always check if default terminal",
            ),
        )
        button = Gtk.Button(label="Set as default terminal")
        button.set_halign(Gtk.Align.START)
        button.connect("clicked", lambda *_args: self._make_default_terminal())
        self._span(grid, row, button)
        return self._scrolled(grid)

    def _shortcuts_page(self) -> Gtk.Widget:
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        box.set_margin_start(12)
        box.set_margin_end(12)
        box.set_margin_top(12)
        box.set_margin_bottom(12)
        enabled = self._check(self.terminal, "shortcuts-enabled", "Enable shortcuts")
        box.pack_start(enabled, False, False, 0)
        store = Gtk.ListStore(str, str, int, int)
        bindings = self._keybindings()
        if bindings is not None:
            schema = bindings.get_property("settings-schema")
            keys = sorted(
                schema.list_keys(),
                key=lambda item: _shortcut_label(schema, item).lower(),
            )
            for key in keys:
                accel_key, mods = _parse_accel(bindings.get_string(key))
                store.append([_shortcut_label(schema, key), key, accel_key, int(mods)])
        view = Gtk.TreeView(model=store)
        view.set_headers_visible(True)
        text = Gtk.CellRendererText()
        column = Gtk.TreeViewColumn("Shortcut", text, text=0)
        column.set_expand(True)
        view.append_column(column)
        accel = Gtk.CellRendererAccel()
        accel.set_property("editable", True)
        accel.connect("accel-edited", self._accel_edited, store)
        accel.connect("accel-cleared", self._accel_cleared, store)
        accel_column = Gtk.TreeViewColumn("Accelerator", accel, accel_key=2, accel_mods=3)
        view.append_column(accel_column)
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scrolled.set_shadow_type(Gtk.ShadowType.IN)
        scrolled.set_vexpand(True)
        scrolled.add(view)
        box.pack_start(scrolled, True, True, 0)
        return box

    def _profile_page(self) -> Gtk.Widget:
        notebook = Gtk.Notebook()
        notebook.append_page(self._text_page(), Gtk.Label(label="Text"))
        notebook.append_page(self._color_page(), Gtk.Label(label="Colors"))
        notebook.append_page(self._scrolling_page(), Gtk.Label(label="Scrolling"))
        notebook.append_page(self._command_page(), Gtk.Label(label="Command"))
        notebook.append_page(self._compatibility_page(), Gtk.Label(label="Compatibility"))
        return notebook

    def _text_page(self) -> Gtk.Widget:
        grid = self._grid()
        row = self._span(grid, 0, self._heading("Text Appearance"))
        self._columns = self._spin_int(16, 511, "default-size-columns", 80)
        self._rows = self._spin_int(4, 511, "default-size-rows", 24)
        size = Gtk.Box(spacing=6)
        size.pack_start(self._columns, False, False, 0)
        size.pack_start(Gtk.Label(label="columns"), False, False, 0)
        size.pack_start(self._rows, False, False, 0)
        size.pack_start(Gtk.Label(label="rows"), False, False, 0)
        reset_size = Gtk.Button(label="Reset")
        reset_size.connect("clicked", lambda *_args: self._reset_size())
        size.pack_start(reset_size, False, False, 0)
        row = self._row(grid, row, "Initial terminal size:", size)
        self._custom_font = Gtk.CheckButton(label="Custom font:")
        self._custom_font.set_active(not self._bool(self.profile, "use-system-font"))
        self._custom_font.connect("toggled", self._on_custom_font)
        self._font = Gtk.FontButton()
        self._font.set_font(self._str(self.profile, "font", "Monospace 12"))
        self._font.connect("font-set", self._on_font)
        fonts = Gtk.Box(spacing=6)
        fonts.pack_start(self._custom_font, False, False, 0)
        fonts.pack_start(self._font, False, False, 0)
        row = self._span(grid, row, fonts)
        self._cell_width = self._spin_float("cell-width-scale")
        self._cell_height = self._spin_float("cell-height-scale")
        spacing = Gtk.Box(spacing=6)
        spacing.pack_start(self._cell_width, False, False, 0)
        spacing.pack_start(Gtk.Label(label="width"), False, False, 0)
        spacing.pack_start(self._cell_height, False, False, 0)
        spacing.pack_start(Gtk.Label(label="height"), False, False, 0)
        reset_cells = Gtk.Button(label="Reset")
        reset_cells.connect("clicked", lambda *_args: self._reset_cells())
        spacing.pack_start(reset_cells, False, False, 0)
        row = self._row(grid, row, "Cell spacing:", spacing)
        row = self._row(
            grid,
            row,
            "Allow blinking text:",
            self._combo(None, "text-blink-mode", _TEXT_BLINK, "always"),
        )
        row = self._span(grid, row, self._heading("Cursor"))
        row = self._row(
            grid,
            row,
            "Cursor shape:",
            self._combo(None, "cursor-shape", _CURSOR_SHAPE, "block"),
        )
        row = self._row(
            grid,
            row,
            "Cursor blinking:",
            self._combo(None, "cursor-blink-mode", _CURSOR_BLINK, "system"),
        )
        row = self._span(grid, row, self._heading("Sound"))
        row = self._span(grid, row, self._check(None, "audible-bell", "Terminal bell"))
        self._profile_id_label = Gtk.Label(xalign=0, selectable=True)
        self._profile_id_label.set_margin_top(18)
        identity = Gtk.Box(spacing=6)
        identity.pack_start(Gtk.Label(label="Profile ID:", xalign=0), False, False, 0)
        identity.pack_start(self._profile_id_label, False, False, 0)
        self._span(grid, row, identity)
        return self._scrolled(grid)

    def _color_page(self) -> Gtk.Widget:
        grid = self._grid()
        row = self._span(grid, 0, self._heading("Text and Background Color"))
        self._theme_colors = self._check(None, "use-theme-colors", "Use colors from system theme")
        self._theme_colors.connect("toggled", lambda *_args: self._refresh_sensitivity())
        row = self._span(grid, row, self._theme_colors)
        self._scheme = self._scheme_combo()
        row = self._row(grid, row, "Built-in schemes:", self._scheme)
        captions = Gtk.Grid(column_spacing=12)
        text = Gtk.Label(label="Text")
        background = Gtk.Label(label="Background")
        captions.attach(text, 0, 0, 1, 1)
        captions.attach(background, 1, 0, 1, 1)
        row = self._place(grid, row, "", captions)
        self._foreground = self._color("foreground-color", "#171421")
        self._background = self._color("background-color", "#ffffff")
        row = self._place(
            grid,
            row,
            "Default color:",
            self._pair(self._foreground, self._background),
        )
        self._bold_enabled = Gtk.CheckButton(label="Bold color:")
        self._bold_enabled.set_active(not self._bool(self.profile, "bold-color-same-as-fg"))
        self._bold_enabled.connect("toggled", self._on_bold_enabled)
        self._bold = self._color("bold-color", "#000000")
        row = self._place(grid, row, "", self._check_colors(self._bold_enabled, self._bold, None))
        self._cursor_colors = self._check(None, "cursor-colors-set", "Cursor color:")
        self._cursor_colors.connect("toggled", lambda *_args: self._refresh_sensitivity())
        self._cursor_foreground = self._color("cursor-foreground-color", "#ffffff")
        self._cursor_background = self._color("cursor-background-color", "#000000")
        row = self._place(
            grid,
            row,
            "",
            self._check_colors(
                self._cursor_colors,
                self._cursor_foreground,
                self._cursor_background,
            ),
        )
        self._highlight_colors = self._check(None, "highlight-colors-set", "Highlight color:")
        self._highlight_colors.connect("toggled", lambda *_args: self._refresh_sensitivity())
        self._highlight_foreground = self._color("highlight-foreground-color", "#ffffff")
        self._highlight_background = self._color("highlight-background-color", "#000000")
        row = self._place(
            grid,
            row,
            "",
            self._check_colors(
                self._highlight_colors,
                self._highlight_foreground,
                self._highlight_background,
            ),
        )
        self._transparent = self._check(
            None,
            "use-transparent-background",
            "Use transparent background",
        )
        self._transparent.connect("toggled", lambda *_args: self._refresh_sensitivity())
        self._transparency = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 0, 100, 1)
        self._transparency.set_draw_value(False)
        self._transparency.set_hexpand(True)
        self._transparency.set_size_request(240, -1)
        percent = self._integer(self.profile, "background-transparency-percent", 50)
        self._transparency.set_value(percent)
        self._transparency.connect("value-changed", self._on_transparency)
        transparent = Gtk.Box(spacing=8)
        transparent.pack_start(self._transparent, False, False, 0)
        transparent.pack_start(Gtk.Label(label="none"), False, False, 0)
        transparent.pack_start(self._transparency, True, True, 0)
        transparent.pack_start(Gtk.Label(label="full"), False, False, 0)
        row = self._span(grid, row, transparent, expand=True)
        self._theme_transparency = self._check(
            None,
            "use-theme-transparency",
            "Use transparency from system theme",
        )
        self._theme_transparency.connect("toggled", lambda *_args: self._refresh_sensitivity())
        row = self._span(grid, row, self._theme_transparency)
        row = self._span(grid, row, self._heading("Palette"))
        self._palette_scheme = self._palette_combo()
        row = self._row(grid, row, "Built-in schemes:", self._palette_scheme)
        row = self._place(grid, row, "Color palette:", self._palette_grid())
        self._span(
            grid,
            row,
            self._check(None, "bold-is-bright", "Show bold text in bright colors"),
        )
        self._select_matching_schemes()
        return self._scrolled(grid)

    def _scrolling_page(self) -> Gtk.Widget:
        grid = self._grid()
        policy = self._str(self.profile, "scrollbar-policy", "always")
        self._scrollbar = Gtk.CheckButton(label="Show scrollbar")
        self._scrollbar.set_active(policy != "never")
        self._scrollbar.connect("toggled", self._on_scrollbar)
        row = self._span(grid, 0, self._scrollbar)
        row = self._span(grid, row, self._check(None, "scroll-on-output", "Scroll on output"))
        row = self._span(grid, row, self._check(None, "scroll-on-keystroke", "Scroll on keystroke"))
        row = self._span(grid, row, self._check(None, "scroll-on-insert", "Scroll on paste"))
        self._limit = Gtk.CheckButton(label="Limit scrollback to:")
        self._limit.set_active(not self._bool(self.profile, "scrollback-unlimited"))
        self._limit.connect("toggled", self._on_limit)
        self._scrollback = self._spin_int(1, 10_000_000, "scrollback-lines", 10000)
        limited = Gtk.Box(spacing=6)
        limited.pack_start(self._limit, False, False, 0)
        limited.pack_start(self._scrollback, False, False, 0)
        limited.pack_start(Gtk.Label(label="lines"), False, False, 0)
        self._span(grid, row, limited)
        return self._scrolled(grid)

    def _command_page(self) -> Gtk.Widget:
        grid = self._grid()
        row = self._span(grid, 0, self._check(None, "login-shell", "Run command as a login shell"))
        self._custom_command_enabled = self._check(
            None,
            "use-custom-command",
            "Run a custom command instead of my shell",
        )
        self._custom_command_enabled.connect("toggled", lambda *_args: self._refresh_sensitivity())
        row = self._span(grid, row, self._custom_command_enabled)
        self._custom_command = Gtk.Entry()
        self._custom_command.set_text(self._str(self.profile, "custom-command", ""))
        self._custom_command.connect("activate", self._on_custom_command)
        self._custom_command.connect("focus-out-event", self._on_custom_command)
        row = self._row(grid, row, "Custom command:", self._custom_command)
        row = self._row(
            grid,
            row,
            "Preserve working directory:",
            self._combo(None, "preserve-working-directory", _PRESERVE, "safe"),
        )
        self._row(
            grid,
            row,
            "When command exits:",
            self._combo(None, "exit-action", _EXIT, "close"),
        )
        return self._scrolled(grid)

    def _compatibility_page(self) -> Gtk.Widget:
        grid = self._grid()
        row = self._row(
            grid,
            0,
            "Backspace key generates:",
            self._combo(None, "backspace-binding", _ERASE, "ascii-delete"),
        )
        row = self._row(
            grid,
            row,
            "Delete key generates:",
            self._combo(None, "delete-binding", _ERASE, "delete-sequence"),
        )
        self._encoding = self._encoding_combo()
        row = self._row(grid, row, "Encoding:", self._encoding)
        row = self._row(
            grid,
            row,
            "Ambiguous-width characters:",
            self._combo(None, "cjk-utf8-ambiguous-width", _CJK, "narrow"),
        )
        reset = Gtk.Button(label="Reset Compatibility Options to Defaults")
        reset.set_halign(Gtk.Align.START)
        reset.connect("clicked", lambda *_args: self._reset_compatibility())
        self._span(grid, row, reset)
        return self._scrolled(grid)

    def _rebuild_profiles(self) -> None:
        for row in list(self._profile_list.get_children()):
            self._profile_list.remove(row)
        self._profile_rows.clear()
        if self.profiles is None:
            return
        default = self.profiles.get_string("default")
        for profile_id in self.profiles.get_strv("list"):
            row = self._profile_row(profile_id, profile_id == default)
            self._profile_rows[profile_id] = row
            self._profile_list.add(row)
        self._profile_list.show_all()

    def _profile_row(self, profile_id: str, default: bool) -> Gtk.ListBoxRow:
        row = Gtk.ListBoxRow()
        row.profile_id = profile_id
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        box.set_margin_top(4)
        box.set_margin_bottom(4)
        box.set_margin_start(8)
        box.set_margin_end(4)
        name = Gtk.Label(label=_profile_name(profile_id), xalign=0)
        name.set_ellipsize(Pango.EllipsizeMode.END)
        name.set_hexpand(True)
        box.pack_start(name, True, True, 0)
        if default:
            mark = Gtk.Image.new_from_icon_name("emblem-ok-symbolic", Gtk.IconSize.MENU)
            mark.set_tooltip_text("Default profile")
            box.pack_start(mark, False, False, 0)
        menu_button = Gtk.MenuButton()
        menu_button.set_relief(Gtk.ReliefStyle.NONE)
        menu_button.add(Gtk.Image.new_from_icon_name("pan-down-symbolic", Gtk.IconSize.MENU))
        menu_button.set_popup(self._profile_menu(profile_id))
        box.pack_end(menu_button, False, False, 0)
        row.add(box)
        return row

    def _profile_menu(self, profile_id: str) -> Gtk.Menu:
        menu = Gtk.Menu()
        for label, callback in (
            ("Rename…", lambda: self._rename_profile(profile_id)),
            ("Clone…", lambda: self._clone_profile(profile_id)),
            ("Set as default", lambda: self._set_default_profile(profile_id)),
            ("Delete…", lambda: self._delete_profile(profile_id)),
        ):
            item = Gtk.MenuItem(label=label)
            item.connect("activate", lambda *_item, func=callback: func())
            menu.append(item)
        menu.show_all()
        return menu

    def _on_global_selected(self, _list, row) -> None:
        if row is None or self._loading:
            return
        self._profile_list.unselect_all()
        self._clear_row_focus(self._profile_list)
        self._stack.set_visible_child_name(row.page_name)
        row.grab_focus()

    def _on_profile_selected(self, _list, row) -> None:
        if row is None or self._loading:
            return
        self._global_list.unselect_all()
        self._clear_row_focus(self._global_list)
        self._show_profile(row.profile_id)
        row.grab_focus()

    def _clear_row_focus(self, box: Gtk.ListBox) -> None:
        child = box.get_focus_child()
        if child is not None:
            child.unset_state_flags(Gtk.StateFlags.FOCUSED)

    def _select_global(self, name: str) -> None:
        self._loading = True
        self._profile_list.unselect_all()
        for row in self._global_list.get_children():
            if getattr(row, "page_name", None) == name:
                self._global_list.select_row(row)
                break
        self._stack.set_visible_child_name(name)
        self._loading = False

    def _show_profile(self, profile_id: str) -> None:
        settings = open_profile(profile_id)
        if settings is None:
            return
        self._loading = True
        self.profile = settings
        self._profile_id = profile_id
        self._profile_id_label.set_text(profile_id)
        self._custom_font.set_active(not self._bool(settings, "use-system-font"))
        self._font.set_font(self._str(settings, "font", "Monospace 12"))
        self._columns.set_value(self._integer(settings, "default-size-columns", 80))
        self._rows.set_value(self._integer(settings, "default-size-rows", 24))
        self._cell_width.set_value(self._real(settings, "cell-width-scale", 1.0))
        self._cell_height.set_value(self._real(settings, "cell-height-scale", 1.0))
        self._set_combo("text-blink-mode", _TEXT_BLINK, "always")
        self._set_combo("cursor-shape", _CURSOR_SHAPE, "block")
        self._set_combo("cursor-blink-mode", _CURSOR_BLINK, "system")
        self._set_check("audible-bell")
        self._set_check("use-theme-colors")
        self._set_color(self._foreground, "foreground-color", "#171421")
        self._set_color(self._background, "background-color", "#ffffff")
        self._bold_enabled.set_active(not self._bool(settings, "bold-color-same-as-fg"))
        self._set_color(self._bold, "bold-color", "#000000")
        self._set_check("cursor-colors-set")
        self._set_color(self._cursor_foreground, "cursor-foreground-color", "#ffffff")
        self._set_color(self._cursor_background, "cursor-background-color", "#000000")
        self._set_check("highlight-colors-set")
        self._set_color(self._highlight_foreground, "highlight-foreground-color", "#ffffff")
        self._set_color(self._highlight_background, "highlight-background-color", "#000000")
        self._set_check("use-transparent-background")
        self._set_check("use-theme-transparency")
        self._transparency.set_value(self._integer(settings, "background-transparency-percent", 50))
        self._set_check("bold-is-bright")
        self._palette = self._stored_palette()
        self._paint_palette()
        self._select_matching_schemes()
        self._scrollbar.set_active(self._str(settings, "scrollbar-policy", "always") != "never")
        self._set_check("scroll-on-output")
        self._set_check("scroll-on-keystroke")
        self._set_check("scroll-on-insert")
        self._limit.set_active(not self._bool(settings, "scrollback-unlimited"))
        self._scrollback.set_value(self._integer(settings, "scrollback-lines", 10000))
        self._set_check("login-shell")
        self._set_check("use-custom-command")
        self._custom_command.set_text(self._str(settings, "custom-command", ""))
        self._set_combo("preserve-working-directory", _PRESERVE, "safe")
        self._set_combo("exit-action", _EXIT, "close")
        self._set_combo("backspace-binding", _ERASE, "ascii-delete")
        self._set_combo("delete-binding", _ERASE, "delete-sequence")
        self._encoding.set_active_id(self._str(settings, "encoding", "UTF-8"))
        self._set_combo("cjk-utf8-ambiguous-width", _CJK, "narrow")
        self._refresh_sensitivity()
        self._stack.set_visible_child_name("profile")
        self._loading = False

    def _new_profile(self) -> None:
        name = self._ask_name("New Profile", "Unnamed")
        if name is None:
            return
        self._create_profile(name, None)

    def _clone_profile(self, profile_id: str) -> None:
        source = _profile_name(profile_id)
        name = self._ask_name("Clone Profile", f"{source} (Copy)")
        if name is None:
            return
        self._create_profile(name, profile_id)

    def _create_profile(self, name: str, source_id: str | None) -> None:
        if self.profiles is None:
            return
        profile_id = str(uuid.uuid4())
        settings = open_profile(profile_id)
        if settings is None:
            return
        if source_id is not None:
            source = open_profile(source_id)
            if source is not None:
                schema = source.get_property("settings-schema")
                for key in schema.list_keys():
                    settings.set_value(key, source.get_value(key))
        settings.set_string("visible-name", name)
        listed = list(self.profiles.get_strv("list"))
        listed.append(profile_id)
        self.profiles.set_strv("list", listed)
        self._rebuild_profiles()
        self._select_profile_row(profile_id)

    def _rename_profile(self, profile_id: str) -> None:
        name = self._ask_name("Rename Profile", _profile_name(profile_id))
        if name is None:
            return
        settings = open_profile(profile_id)
        if settings is not None:
            settings.set_string("visible-name", name)
        self._rebuild_profiles()
        self._select_profile_row(profile_id)

    def _set_default_profile(self, profile_id: str) -> None:
        if self.profiles is None:
            return
        self.profiles.set_string("default", profile_id)
        self._rebuild_profiles()
        self._select_profile_row(profile_id)

    def _delete_profile(self, profile_id: str) -> None:
        if self.profiles is None:
            return
        listed = list(self.profiles.get_strv("list"))
        if len(listed) <= 1 or profile_id not in listed:
            return
        listed.remove(profile_id)
        if self.profiles.get_string("default") == profile_id:
            self.profiles.set_string("default", listed[0])
        self.profiles.set_strv("list", listed)
        self._rebuild_profiles()
        self._select_global("general")

    def _select_profile_row(self, profile_id: str) -> None:
        row = self._profile_rows.get(profile_id)
        if row is None:
            return
        self._loading = True
        self._global_list.unselect_all()
        self._profile_list.select_row(row)
        self._loading = False
        self._show_profile(profile_id)

    def _ask_name(self, title: str, current: str) -> str | None:
        dialog = Gtk.Dialog(title=title, transient_for=self, modal=True)
        dialog.add_button("Cancel", Gtk.ResponseType.CANCEL)
        dialog.add_button("OK", Gtk.ResponseType.OK)
        dialog.set_default_response(Gtk.ResponseType.OK)
        entry = Gtk.Entry()
        entry.set_text(current)
        entry.set_activates_default(True)
        area = dialog.get_content_area()
        area.set_margin_start(12)
        area.set_margin_end(12)
        area.set_margin_top(12)
        area.set_margin_bottom(12)
        area.add(entry)
        dialog.show_all()
        response = dialog.run()
        name = entry.get_text().strip()
        dialog.destroy()
        if response != Gtk.ResponseType.OK or not name:
            return None
        return name

    def _make_default_terminal(self) -> None:
        info = Gio.DesktopAppInfo.new("org.gnome.Terminal.desktop")
        if info is None:
            self._show_error("GNOME Terminal is not available.")
            return
        try:
            info.set_as_default_for_type("x-scheme-handler/terminal")
        except GLib.Error as error:
            self._show_error(error.message)

    def _scheme_combo(self) -> Gtk.ComboBoxText:
        combo = Gtk.ComboBoxText()
        combo.append("custom", "Custom")
        for name, _foreground, _background in _COLOR_SCHEMES:
            combo.append(name, name)
        combo.connect("changed", self._on_scheme)
        return combo

    def _palette_combo(self) -> Gtk.ComboBoxText:
        combo = Gtk.ComboBoxText()
        combo.append("custom", "Custom")
        for name, _colors in _PALETTES:
            combo.append(name, name)
        combo.connect("changed", self._on_palette_scheme)
        return combo

    def _encoding_combo(self) -> Gtk.ComboBoxText:
        combo = Gtk.ComboBoxText()
        current = self._str(self.profile, "encoding", "UTF-8")
        seen = set()
        for charset, language in _ENCODINGS:
            combo.append(charset, f"{language} — {charset}")
            seen.add(charset)
        if current not in seen:
            combo.append(current, current)
        combo.set_active_id(current)
        combo.connect("changed", self._on_encoding)
        return combo

    def _palette_grid(self) -> Gtk.Grid:
        self._palette = self._stored_palette()
        self._palette_buttons: list[Gtk.ColorButton] = []
        grid = Gtk.Grid(column_spacing=4, row_spacing=4)
        for index in range(16):
            button = Gtk.ColorButton()
            button.connect(
                "color-set",
                lambda widget, item=index: self._set_palette_color(item, widget),
            )
            self._palette_buttons.append(button)
            grid.attach(button, index % 8, index // 8, 1, 1)
        self._paint_palette()
        return grid

    def _paint_palette(self) -> None:
        for index, button in enumerate(self._palette_buttons):
            color = Gdk.RGBA()
            color.parse(self._palette[index] if index < len(self._palette) else "#000000")
            button.set_rgba(color)

    def _select_matching_schemes(self) -> None:
        foreground = _norm(self._str(self.profile, "foreground-color", ""))
        background = _norm(self._str(self.profile, "background-color", ""))
        match = "custom"
        for name, scheme_fg, scheme_bg in _COLOR_SCHEMES:
            if foreground == scheme_fg and background == scheme_bg:
                match = name
                break
        self._scheme.set_active_id(match)
        colors = [_norm(item) for item in self._palette[:16]]
        palette_match = "custom"
        for name, palette in _PALETTES:
            if colors == list(palette):
                palette_match = name
                break
        self._palette_scheme.set_active_id(palette_match)

    def _on_scheme(self, combo: Gtk.ComboBoxText) -> None:
        if self._loading or self.profile is None:
            return
        name = combo.get_active_id()
        for scheme, foreground, background in _COLOR_SCHEMES:
            if scheme == name:
                self._write_string(self.profile, "foreground-color", foreground)
                self._write_string(self.profile, "background-color", background)
                self._write_bool(self.profile, "use-theme-colors", False)
                self._loading = True
                self._theme_colors.set_active(False)
                self._set_color(self._foreground, "foreground-color", foreground)
                self._set_color(self._background, "background-color", background)
                self._refresh_sensitivity()
                self._loading = False
                return

    def _on_palette_scheme(self, combo: Gtk.ComboBoxText) -> None:
        if self._loading or self.profile is None:
            return
        name = combo.get_active_id()
        for scheme, colors in _PALETTES:
            if scheme == name:
                self._palette = list(colors)
                self._write_strv("palette", self._palette)
                self._loading = True
                self._paint_palette()
                self._loading = False
                return

    def _set_palette_color(self, index: int, button: Gtk.ColorButton) -> None:
        if self._loading or index >= len(self._palette):
            return
        colors = list(self._palette)
        colors[index] = rgba_to_hex(button.get_rgba())
        self._palette = colors
        self._write_strv("palette", colors)
        self._loading = True
        self._palette_scheme.set_active_id("custom")
        self._loading = False

    def _on_custom_font(self, button: Gtk.CheckButton) -> None:
        self._write_bool(self.profile, "use-system-font", not button.get_active())
        self._refresh_sensitivity()

    def _on_font(self, button: Gtk.FontButton) -> None:
        self._write_string(self.profile, "font", button.get_font())

    def _on_bold_enabled(self, button: Gtk.CheckButton) -> None:
        self._write_bool(self.profile, "bold-color-same-as-fg", not button.get_active())
        self._refresh_sensitivity()

    def _on_scrollbar(self, button: Gtk.CheckButton) -> None:
        policy = "always" if button.get_active() else "never"
        self._write_string(self.profile, "scrollbar-policy", policy)

    def _on_limit(self, button: Gtk.CheckButton) -> None:
        self._write_bool(self.profile, "scrollback-unlimited", not button.get_active())
        self._refresh_sensitivity()

    def _on_transparency(self, scale: Gtk.Scale) -> None:
        self._write_int(self.profile, "background-transparency-percent", int(scale.get_value()))

    def _on_custom_command(self, entry: Gtk.Entry, *_args) -> bool:
        self._write_string(self.profile, "custom-command", entry.get_text())
        return False

    def _on_encoding(self, combo: Gtk.ComboBoxText) -> None:
        active = combo.get_active_id()
        if active:
            self._write_string(self.profile, "encoding", active)

    def _reset_size(self) -> None:
        self._write_int(self.profile, "default-size-columns", 80)
        self._write_int(self.profile, "default-size-rows", 24)
        self._loading = True
        self._columns.set_value(80)
        self._rows.set_value(24)
        self._loading = False

    def _reset_cells(self) -> None:
        self._write_double_value("cell-width-scale", 1.0)
        self._write_double_value("cell-height-scale", 1.0)
        self._loading = True
        self._cell_width.set_value(1)
        self._cell_height.set_value(1)
        self._loading = False

    def _reset_compatibility(self) -> None:
        self._write_string(self.profile, "backspace-binding", "ascii-delete")
        self._write_string(self.profile, "delete-binding", "delete-sequence")
        self._write_string(self.profile, "encoding", "UTF-8")
        self._write_string(self.profile, "cjk-utf8-ambiguous-width", "narrow")
        self._loading = True
        self._set_combo("backspace-binding", _ERASE, "ascii-delete")
        self._set_combo("delete-binding", _ERASE, "delete-sequence")
        self._encoding.set_active_id("UTF-8")
        self._set_combo("cjk-utf8-ambiguous-width", _CJK, "narrow")
        self._loading = False

    def _accel_edited(self, _renderer, path, key, mods, _hardware, store) -> None:
        bindings = self._keybindings()
        if bindings is None:
            return
        iterator = store.get_iter(path)
        setting_key = store.get_value(iterator, 1)
        name = Gtk.accelerator_name(key, mods) or "disabled"
        self._write_string(bindings, setting_key, name)
        store.set_value(iterator, 2, key)
        store.set_value(iterator, 3, int(mods))

    def _accel_cleared(self, _renderer, path, store) -> None:
        bindings = self._keybindings()
        if bindings is None:
            return
        iterator = store.get_iter(path)
        setting_key = store.get_value(iterator, 1)
        self._write_string(bindings, setting_key, "disabled")
        store.set_value(iterator, 2, 0)
        store.set_value(iterator, 3, 0)

    def _refresh_sensitivity(self) -> None:
        if not hasattr(self, "_font"):
            return
        self._font.set_sensitive(self._custom_font.get_active())
        custom = not self._theme_colors.get_active()
        self._foreground.set_sensitive(custom)
        self._background.set_sensitive(custom)
        self._bold.set_sensitive(custom and self._bold_enabled.get_active())
        self._cursor_foreground.set_sensitive(self._cursor_colors.get_active())
        self._cursor_background.set_sensitive(self._cursor_colors.get_active())
        self._highlight_foreground.set_sensitive(self._highlight_colors.get_active())
        self._highlight_background.set_sensitive(self._highlight_colors.get_active())
        self._transparency.set_sensitive(
            self._transparent.get_active() and not self._theme_transparency.get_active()
        )
        self._scrollback.set_sensitive(self._limit.get_active())
        self._custom_command.set_sensitive(self._custom_command_enabled.get_active())

    def _check(self, settings, key: str, label: str) -> Gtk.CheckButton:
        button = Gtk.CheckButton(label=label)
        button.set_active(self._bool(settings if settings is not None else self.profile, key))
        button.connect(
            "toggled",
            lambda widget, item=key, target=settings: self._toggled(target, item, widget),
        )
        button.setting_key = key
        return button

    def _toggled(self, settings, key: str, button: Gtk.CheckButton) -> None:
        target = self.profile if settings is None else settings
        self._write_bool(target, key, button.get_active())

    def _combo(
        self,
        settings,
        key: str,
        options: tuple[tuple[str, str], ...],
        fallback: str,
    ) -> Gtk.ComboBoxText:
        combo = Gtk.ComboBoxText()
        for nick, label in options:
            combo.append(nick, label)
        target = self.profile if settings is None else settings
        current = self._str(target, key, fallback)
        if current and all(nick != current for nick, _label in options):
            combo.append(current, current)
        if current:
            combo.set_active_id(current)
        combo.setting_key = key
        combo.setting_target = settings
        combo.connect(
            "changed",
            lambda widget, item=key, owner=settings: self._choice(owner, item, widget),
        )
        return combo

    def _choice(self, settings, key: str, combo: Gtk.ComboBoxText) -> None:
        active = combo.get_active_id()
        if not active:
            return
        target = self.profile if settings is None else settings
        self._write_string(target, key, active)

    def _set_combo(self, key: str, options: tuple[tuple[str, str], ...], fallback: str) -> None:
        value = self._str(self.profile, key, fallback)
        for child in self._walk():
            if isinstance(child, Gtk.ComboBoxText) and getattr(child, "setting_key", None) == key:
                if child.get_active_id() != value:
                    child.set_active_id(value)
                return

    def _set_check(self, key: str) -> None:
        value = self._bool(self.profile, key)
        for child in self._walk():
            if isinstance(child, Gtk.CheckButton) and getattr(child, "setting_key", None) == key:
                if child.get_active() != value:
                    child.set_active(value)
                return

    def _walk(self):
        stack = [self._stack]
        while stack:
            widget = stack.pop()
            yield widget
            if isinstance(widget, Gtk.Container):
                stack.extend(widget.get_children())

    def _color(self, key: str, fallback: str) -> Gtk.ColorButton:
        button = Gtk.ColorButton()
        self._set_color(button, key, fallback)
        button.connect("color-set", lambda widget, item=key: self._write_color(item, widget))
        return button

    def _set_color(self, button: Gtk.ColorButton, key: str, fallback: str) -> None:
        color = Gdk.RGBA()
        if not color.parse(self._str(self.profile, key, fallback)):
            color.parse(fallback)
        button.set_rgba(color)

    def _write_color(self, key: str, button: Gtk.ColorButton) -> None:
        self._write_string(self.profile, key, rgba_to_hex(button.get_rgba()))

    def _spin_int(self, lower: int, upper: int, key: str, fallback: int) -> Gtk.SpinButton:
        spin = Gtk.SpinButton.new_with_range(lower, upper, 1)
        spin.set_value(self._integer(self.profile, key, fallback))
        spin.connect(
            "value-changed",
            lambda widget, item=key: self._write_int(self.profile, item, int(widget.get_value())),
        )
        return spin

    def _spin_float(self, key: str) -> Gtk.SpinButton:
        spin = Gtk.SpinButton.new_with_range(1.0, 2.0, 0.1)
        spin.set_digits(2)
        spin.set_value(self._real(self.profile, key, 1.0))
        spin.connect(
            "value-changed",
            lambda widget, item=key: self._write_double_value(item, widget.get_value()),
        )
        return spin

    def _stored_palette(self) -> list[str]:
        if self.profile is None:
            return ["#000000"] * 16
        try:
            stored = list(self.profile.get_strv("palette"))
        except GLib.Error:
            stored = []
        if len(stored) < 16:
            stored = list(_PALETTES[0][1])
        return stored

    def _write_bool(self, settings, key: str, value: bool) -> None:
        if self._loading or settings is None:
            return
        try:
            if bool(settings.get_boolean(key)) == value:
                return
            settings.set_boolean(key, value)
        except GLib.Error as error:
            self._show_error(error.message)

    def _write_string(self, settings, key: str, value: str) -> None:
        if self._loading or settings is None:
            return
        try:
            if settings.get_string(key) == value:
                return
            settings.set_string(key, value)
        except GLib.Error as error:
            self._show_error(error.message)

    def _write_int(self, settings, key: str, value: int) -> None:
        if self._loading or settings is None:
            return
        try:
            if int(settings.get_int(key)) == value:
                return
            settings.set_int(key, value)
        except GLib.Error as error:
            self._show_error(error.message)

    def _write_double_value(self, key: str, value: float) -> None:
        if self._loading or self.profile is None:
            return
        try:
            if abs(float(self.profile.get_double(key)) - value) < 0.001:
                return
            self.profile.set_double(key, value)
        except GLib.Error as error:
            self._show_error(error.message)

    def _write_strv(self, key: str, values: list[str]) -> None:
        if self._loading or self.profile is None:
            return
        try:
            current = [_norm(item) for item in self.profile.get_strv(key)]
            if current == [_norm(item) for item in values]:
                return
            self.profile.set_strv(key, values)
        except GLib.Error as error:
            self._show_error(error.message)

    def _show_error(self, message: str) -> None:
        dialog = Gtk.MessageDialog(
            transient_for=self,
            modal=True,
            message_type=Gtk.MessageType.ERROR,
            buttons=Gtk.ButtonsType.CLOSE,
            text="Could not save the setting",
        )
        dialog.format_secondary_text(message)
        dialog.run()
        dialog.destroy()

    def _keybindings(self):
        if self.terminal is None:
            return None
        try:
            return self.terminal.get_child("keybindings")
        except GLib.Error:
            return None

    def _bool(self, settings, key: str) -> bool:
        if settings is None:
            return False
        try:
            return bool(settings.get_boolean(key))
        except GLib.Error:
            return False

    def _str(self, settings, key: str, fallback: str) -> str:
        if settings is None:
            return fallback
        try:
            value = settings.get_string(key)
        except GLib.Error:
            return fallback
        return value or fallback

    def _integer(self, settings, key: str, fallback: int) -> int:
        if settings is None:
            return fallback
        try:
            return int(settings.get_int(key))
        except GLib.Error:
            return fallback

    def _real(self, settings, key: str, fallback: float) -> float:
        if settings is None:
            return fallback
        try:
            return float(settings.get_double(key))
        except GLib.Error:
            return fallback

    def _grid(self) -> Gtk.Grid:
        grid = Gtk.Grid(column_spacing=12, row_spacing=8)
        grid.set_margin_start(12)
        grid.set_margin_end(12)
        grid.set_margin_top(12)
        grid.set_margin_bottom(12)
        return grid

    def _scrolled(self, grid: Gtk.Grid) -> Gtk.ScrolledWindow:
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scrolled.add(grid)
        return scrolled

    def _heading(self, text: str) -> Gtk.Label:
        label = Gtk.Label(xalign=0)
        label.set_markup(f"<b>{GLib.markup_escape_text(text)}</b>")
        label.set_margin_top(6)
        return label

    def _row(self, grid: Gtk.Grid, row: int, label: str, widget: Gtk.Widget) -> int:
        text = Gtk.Label(label=label, xalign=0)
        widget.set_hexpand(True)
        grid.attach(text, 0, row, 1, 1)
        grid.attach(widget, 1, row, 1, 1)
        return row + 1

    def _place(self, grid: Gtk.Grid, row: int, label: str, widget: Gtk.Widget) -> int:
        if label:
            return self._row(grid, row, label, widget)
        widget.set_halign(Gtk.Align.START)
        grid.attach(widget, 1, row, 1, 1)
        return row + 1

    def _span(self, grid: Gtk.Grid, row: int, widget: Gtk.Widget, *, expand: bool = False) -> int:
        if expand:
            widget.set_hexpand(True)
            widget.set_halign(Gtk.Align.FILL)
        else:
            widget.set_halign(Gtk.Align.START)
        grid.attach(widget, 0, row, 2, 1)
        return row + 1

    def _pair(self, first: Gtk.Widget, second: Gtk.Widget) -> Gtk.Grid:
        grid = Gtk.Grid(column_spacing=12)
        grid.attach(first, 0, 0, 1, 1)
        grid.attach(second, 1, 0, 1, 1)
        return grid

    def _check_colors(
        self,
        check: Gtk.Widget,
        first: Gtk.Widget,
        second: Gtk.Widget | None,
    ) -> Gtk.Box:
        box = Gtk.Box(spacing=12)
        box.pack_start(check, False, False, 0)
        box.pack_start(first, False, False, 0)
        if second is not None:
            box.pack_start(second, False, False, 0)
        return box


def _profiles_list():
    try:
        return Gio.Settings.new("org.gnome.Terminal.ProfilesList")
    except GLib.Error:
        return None


def _profile_name(profile_id: str) -> str:
    settings = open_profile(profile_id)
    if settings is None:
        return "Unnamed"
    try:
        name = settings.get_string("visible-name")
    except GLib.Error:
        return "Unnamed"
    return name or "Unnamed"


def _shortcut_label(schema, key: str) -> str:
    try:
        summary = schema.get_key(key).get_summary() or key
    except (AttributeError, GLib.Error):
        summary = key
    prefix = "Keyboard shortcut to "
    if summary.startswith(prefix):
        summary = summary[len(prefix) :]
        summary = summary[:1].upper() + summary[1:]
    return summary


def _parse_accel(value: str) -> tuple[int, int]:
    if not value or value == "disabled":
        return 0, 0
    key, mods = Gtk.accelerator_parse(value)
    return int(key), int(mods)


def _norm(color: str) -> str:
    parsed = Gdk.RGBA()
    if parsed.parse(color):
        return rgba_to_hex(parsed)
    return color.strip().lower()
