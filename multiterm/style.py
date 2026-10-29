"""Colors and terminal settings taken from the GNOME Terminal profile.

Ubuntu ships GNOME Terminal with ``theme-variant=dark`` even when the desktop
theme is the light Yaru theme. ``use-theme-colors`` then means "colors of that
terminal variant", not the colors of the surrounding file manager.
"""

from __future__ import annotations

import contextlib
import warnings
from typing import Any

import gi

gi.require_version("Gdk", "3.0")
gi.require_version("Gtk", "3.0")
gi.require_version("Vte", "2.91")

from gi.repository import Gdk, Gio, GLib, Gtk, Pango, Vte

# Palette shipped with GNOME Terminal. Used only when the profile cannot be read.
FALLBACK_PALETTE = (
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
    "#33da7a",
    "#e9ad0c",
    "#2a7bde",
    "#c061cb",
    "#33c7de",
    "#ffffff",
)

_CURSOR_BLINK = {
    "system": Vte.CursorBlinkMode.SYSTEM,
    "on": Vte.CursorBlinkMode.ON,
    "off": Vte.CursorBlinkMode.OFF,
}
_CURSOR_SHAPE = {
    "block": Vte.CursorShape.BLOCK,
    "ibeam": Vte.CursorShape.IBEAM,
    "underline": Vte.CursorShape.UNDERLINE,
}
_TEXT_BLINK = {
    "always": Vte.TextBlinkMode.ALWAYS,
    "never": Vte.TextBlinkMode.NEVER,
    "focused": Vte.TextBlinkMode.FOCUSED,
    "unfocused": Vte.TextBlinkMode.UNFOCUSED,
}
_ERASE = {
    "auto": Vte.EraseBinding.AUTO,
    "ascii-backspace": Vte.EraseBinding.ASCII_BACKSPACE,
    "ascii-delete": Vte.EraseBinding.ASCII_DELETE,
    "delete-sequence": Vte.EraseBinding.DELETE_SEQUENCE,
    "tty": Vte.EraseBinding.TTY,
}


class TerminalLook:
    """The visual settings applied to every new terminal widget."""

    def __init__(self, profile: Any) -> None:
        font_name = (
            system_monospace_font()
            if _boolean(profile, "use-system-font", True)
            else _string(profile, "font", "Monospace 12")
        )
        self.font = Pango.FontDescription.from_string(font_name)
        self.palette = [_rgba(spec) for spec in _palette(profile)]
        self.foreground, self.background = _foreground_background(profile)
        if _boolean(profile, "bold-color-same-as-fg", True):
            self.bold = self.foreground
        else:
            self.bold = _rgba(_string(profile, "bold-color", "#000000"))
        self.bold_is_bright = _boolean(profile, "bold-is-bright", False)
        self.cursor_colors = _boolean(profile, "cursor-colors-set", False)
        self.cursor_background = _rgba(_string(profile, "cursor-background-color", "#000000"))
        self.cursor_foreground = _rgba(_string(profile, "cursor-foreground-color", "#ffffff"))
        self.highlight_colors = _boolean(profile, "highlight-colors-set", False)
        self.highlight_background = _rgba(_string(profile, "highlight-background-color", "#000000"))
        self.highlight_foreground = _rgba(_string(profile, "highlight-foreground-color", "#ffffff"))
        self.scrollback_lines = _int(profile, "scrollback-lines", 10000)
        self.scroll_on_keystroke = _boolean(profile, "scroll-on-keystroke", True)
        self.scroll_on_output = _boolean(profile, "scroll-on-output", False)
        self.scroll_on_insert = _boolean(profile, "scroll-on-insert", True)
        self.audible_bell = _boolean(profile, "audible-bell", True)
        self.cell_width_scale = _double(profile, "cell-width-scale", 1.0)
        self.cell_height_scale = _double(profile, "cell-height-scale", 1.0)
        self.enable_bidi = _boolean(profile, "enable-bidi", True)
        self.enable_shaping = _boolean(profile, "enable-shaping", True)
        self.enable_sixel = _boolean(profile, "enable-sixel", False)
        self.cursor_blink = _string(profile, "cursor-blink-mode", "system")
        self.cursor_shape = _string(profile, "cursor-shape", "block")
        self.text_blink = _string(profile, "text-blink-mode", "always")
        self.backspace_binding = _string(profile, "backspace-binding", "ascii-delete")
        self.delete_binding = _string(profile, "delete-binding", "delete-sequence")
        width = _string(profile, "cjk-utf8-ambiguous-width", "narrow")
        self.cjk_ambiguous_width = 2 if width == "wide" else 1
        self.scrollbar_policy = _string(profile, "scrollbar-policy", "always")
        self.scrollback_unlimited = _boolean(profile, "scrollback-unlimited", False)
        self.login_shell = _boolean(profile, "login-shell", False)
        self.use_custom_command = _boolean(profile, "use-custom-command", False)
        self.custom_command = _string(profile, "custom-command", "")
        self.exit_action = _string(profile, "exit-action", "close")
        self.preserve_directory = _string(profile, "preserve-working-directory", "safe")
        self.encoding = _string(profile, "encoding", "UTF-8")
        self.rewrap_on_resize = _boolean(profile, "rewrap-on-resize", True)
        self.word_char_exceptions = _maybe_string(profile, "word-char-exceptions")
        self.use_transparent_background = _boolean(profile, "use-transparent-background", False)
        self.use_theme_transparency = _boolean(profile, "use-theme-transparency", True)
        self.transparency_percent = _int(profile, "background-transparency-percent", 50)


def load_terminal_look() -> TerminalLook:
    return TerminalLook(_gnome_profile())


def apply_terminal_look(terminal: Vte.Terminal, look: TerminalLook) -> None:
    """Copy ``look`` onto a VTE widget. Safe to call again after a theme change."""
    if getattr(terminal, "_applying_look", False):
        return
    terminal._applying_look = True
    try:
        terminal.set_font(look.font)
        terminal.set_colors(look.foreground, _background(look), look.palette)
        terminal.set_color_bold(look.bold)
        terminal.set_bold_is_bright(look.bold_is_bright)
        if look.cursor_colors:
            terminal.set_color_cursor(look.cursor_background)
            terminal.set_color_cursor_foreground(look.cursor_foreground)
        else:
            terminal.set_color_cursor(None)
            terminal.set_color_cursor_foreground(None)
        if look.highlight_colors:
            terminal.set_color_highlight(look.highlight_background)
            terminal.set_color_highlight_foreground(look.highlight_foreground)
        else:
            terminal.set_color_highlight(None)
            terminal.set_color_highlight_foreground(None)
        if look.scrollback_unlimited:
            terminal.set_scrollback_lines(-1)
        else:
            terminal.set_scrollback_lines(look.scrollback_lines)
        terminal.set_scroll_on_keystroke(look.scroll_on_keystroke)
        terminal.set_scroll_on_output(look.scroll_on_output)
        terminal.set_scroll_on_insert(look.scroll_on_insert)
        terminal.set_audible_bell(look.audible_bell)
        terminal.set_cell_width_scale(look.cell_width_scale)
        terminal.set_cell_height_scale(look.cell_height_scale)
        terminal.set_enable_bidi(look.enable_bidi)
        terminal.set_enable_shaping(look.enable_shaping)
        terminal.set_enable_sixel(look.enable_sixel)
        terminal.set_cursor_blink_mode(
            _CURSOR_BLINK.get(look.cursor_blink, Vte.CursorBlinkMode.SYSTEM)
        )
        terminal.set_cursor_shape(_CURSOR_SHAPE.get(look.cursor_shape, Vte.CursorShape.BLOCK))
        terminal.set_text_blink_mode(_TEXT_BLINK.get(look.text_blink, Vte.TextBlinkMode.ALWAYS))
        terminal.set_backspace_binding(
            _ERASE.get(look.backspace_binding, Vte.EraseBinding.ASCII_DELETE)
        )
        terminal.set_delete_binding(
            _ERASE.get(look.delete_binding, Vte.EraseBinding.DELETE_SEQUENCE)
        )
        terminal.set_cjk_ambiguous_width(look.cjk_ambiguous_width)
        with contextlib.suppress(Exception):
            terminal.set_word_char_exceptions(look.word_char_exceptions)
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", DeprecationWarning)
            with contextlib.suppress(Exception):
                terminal.set_encoding(look.encoding or "UTF-8")
            with contextlib.suppress(Exception):
                terminal.set_rewrap_on_resize(look.rewrap_on_resize)
    finally:
        terminal._applying_look = False


def scrollbar_policy(name: str) -> tuple[Gtk.PolicyType, bool]:
    """Vertical scrollbar policy and whether it may draw over the text.

    The second value is Gtk.ScrolledWindow overlay scrolling. GNOME Terminal's
    "always" policy is a real scrollbar, not an overlay.
    """
    if name == "never":
        return Gtk.PolicyType.NEVER, False
    if name == "overlay":
        return Gtk.PolicyType.AUTOMATIC, True
    return Gtk.PolicyType.ALWAYS, False


def terminal_theme_variant() -> str:
    """``dark``, ``light``, or ``system``, from GNOME Terminal's preferences."""
    try:
        value = Gio.Settings.new("org.gnome.Terminal.Legacy.Settings").get_string("theme-variant")
    except GLib.Error:
        return "system"
    if value in ("dark", "light", "system"):
        return value
    return "system"


def terminal_prefers_dark() -> bool:
    variant = terminal_theme_variant()
    if variant == "dark":
        return True
    if variant == "light":
        return False
    try:
        scheme = Gio.Settings.new("org.gnome.desktop.interface").get_string("color-scheme")
    except GLib.Error:
        return False
    return scheme == "prefer-dark"


def apply_terminal_theme_preference() -> None:
    """Ask GTK for the same light/dark variant GNOME Terminal is using."""
    settings = Gtk.Settings.get_default()
    if settings is not None:
        settings.set_property("gtk-application-prefer-dark-theme", terminal_prefers_dark())


def _foreground_background(profile: Any) -> tuple[Gdk.RGBA, Gdk.RGBA]:
    if not _boolean(profile, "use-theme-colors", True):
        return (
            _rgba(_string(profile, "foreground-color", "#171421")),
            _rgba(_string(profile, "background-color", "#ffffff")),
        )
    apply_terminal_theme_preference()
    if terminal_prefers_dark():
        foreground = _theme_color("theme_fg_color") or _rgba("#f7f7f7")
        background = _theme_color("theme_base_color") or _rgba("#272727")
    else:
        foreground = _theme_color("theme_fg_color") or _rgba("#3d3d3d")
        background = _theme_color("theme_base_color") or _rgba("#ffffff")
    return foreground, background


def _palette(profile: Any) -> list[str]:
    if profile is None:
        return list(FALLBACK_PALETTE)
    try:
        stored = list(profile.get_strv("palette"))
    except GLib.Error:
        return list(FALLBACK_PALETTE)
    if len(stored) in (8, 16, 232, 256):
        return stored
    return list(FALLBACK_PALETTE)


def gnome_profile_settings() -> Any:
    """The default GNOME Terminal profile, or None when it cannot be opened."""
    return _gnome_profile()


def terminal_settings() -> Any:
    """Global GNOME Terminal settings, including the light/dark variant."""
    try:
        return Gio.Settings.new("org.gnome.Terminal.Legacy.Settings")
    except GLib.Error:
        return None


def rgba_to_hex(color: Gdk.RGBA) -> str:
    """HTML color. GNOME Terminal stores palette entries in this form."""
    red = round(max(0.0, min(1.0, color.red)) * 255)
    green = round(max(0.0, min(1.0, color.green)) * 255)
    blue = round(max(0.0, min(1.0, color.blue)) * 255)
    return f"#{red:02x}{green:02x}{blue:02x}"


def _background(look: TerminalLook) -> Gdk.RGBA:
    """Background copy. Transparency is an alpha on that color, not a second setting."""
    color = _copy_rgba(look.background)
    if look.use_transparent_background and not look.use_theme_transparency:
        color.alpha = max(0.0, min(1.0, 1.0 - (look.transparency_percent / 100.0)))
    else:
        color.alpha = 1.0
    return color


def _copy_rgba(color: Gdk.RGBA) -> Gdk.RGBA:
    copied = Gdk.RGBA()
    copied.red = color.red
    copied.green = color.green
    copied.blue = color.blue
    copied.alpha = color.alpha
    return copied


def _gnome_profile() -> Any:
    try:
        profiles = Gio.Settings.new("org.gnome.Terminal.ProfilesList")
        default = profiles.get_string("default")
    except GLib.Error:
        return None
    if not default:
        return None
    try:
        return Gio.Settings.new_with_path(
            "org.gnome.Terminal.Legacy.Profile",
            f"/org/gnome/terminal/legacy/profiles:/{default}/",
        )
    except GLib.Error:
        return None


def _theme_color(name: str) -> Gdk.RGBA | None:
    found, color = Gtk.Label().get_style_context().lookup_color(name)
    if found:
        return color
    return None


def _rgba(spec: str) -> Gdk.RGBA:
    color = Gdk.RGBA()
    if not color.parse(spec):
        color.parse("#000000")
    return color


def system_monospace_font() -> str:
    try:
        name = Gio.Settings.new("org.gnome.desktop.interface").get_string("monospace-font-name")
    except GLib.Error:
        name = ""
    return name or "Monospace 12"


def _maybe_string(profile: Any, key: str) -> str:
    if profile is None:
        return ""
    try:
        unpacked = profile.get_value(key).unpack()
    except (GLib.Error, AttributeError):
        return ""
    return unpacked if isinstance(unpacked, str) else ""


def _string(profile: Any, key: str, fallback: str) -> str:
    if profile is None:
        return fallback
    try:
        value = profile.get_string(key)
    except GLib.Error:
        return fallback
    return value or fallback


def _boolean(profile: Any, key: str, fallback: bool) -> bool:
    if profile is None:
        return fallback
    try:
        return bool(profile.get_boolean(key))
    except GLib.Error:
        return fallback


def _double(profile: Any, key: str, fallback: float) -> float:
    if profile is None:
        return fallback
    try:
        return float(profile.get_double(key))
    except GLib.Error:
        return fallback


def _int(profile: Any, key: str, fallback: int) -> int:
    if profile is None:
        return fallback
    try:
        return int(profile.get_int(key))
    except GLib.Error:
        return fallback
