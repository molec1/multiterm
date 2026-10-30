//! Colors and terminal settings taken from the GNOME Terminal profile.

use gio::prelude::*;
use gtk::gdk;
use gtk::pango;
use gtk::prelude::*;
use vte::TerminalExt;

const FALLBACK_PALETTE: [&str; 16] = [
    "#171421", "#c01c28", "#26a269", "#a2734c", "#12488b", "#a347ba", "#2aa1b3", "#d0cfcc",
    "#5e5c64", "#f66151", "#33da7a", "#e9ad0c", "#2a7bde", "#c061cb", "#33c7de", "#ffffff",
];

#[derive(Clone)]
pub struct TerminalLook {
    pub font: pango::FontDescription,
    pub palette: Vec<gdk::RGBA>,
    pub foreground: gdk::RGBA,
    pub background: gdk::RGBA,
    pub bold: gdk::RGBA,
    pub bold_is_bright: bool,
    pub cursor_colors: bool,
    pub cursor_background: gdk::RGBA,
    pub cursor_foreground: gdk::RGBA,
    pub highlight_colors: bool,
    pub highlight_background: gdk::RGBA,
    pub highlight_foreground: gdk::RGBA,
    pub scrollback_lines: i64,
    pub scrollback_unlimited: bool,
    pub scroll_on_keystroke: bool,
    pub scroll_on_output: bool,
    pub scroll_on_insert: bool,
    pub audible_bell: bool,
    pub cell_width_scale: f64,
    pub cell_height_scale: f64,
    pub enable_bidi: bool,
    pub enable_shaping: bool,
    pub enable_sixel: bool,
    pub cursor_blink: String,
    pub cursor_shape: String,
    pub text_blink: String,
    pub backspace_binding: String,
    pub delete_binding: String,
    pub cjk_ambiguous_width: i32,
    pub scrollbar_policy: String,
    pub login_shell: bool,
    pub use_custom_command: bool,
    pub custom_command: String,
    pub exit_action: String,
    pub preserve_directory: String,
    pub word_char_exceptions: String,
    pub use_transparent_background: bool,
    pub use_theme_transparency: bool,
    pub transparency_percent: i32,
}

pub fn load_terminal_look() -> TerminalLook {
    TerminalLook::from_profile(gnome_profile_settings().as_ref())
}

impl TerminalLook {
    fn from_profile(profile: Option<&gio::Settings>) -> Self {
        let font_name = if boolean(profile, "use-system-font", true) {
            system_monospace_font()
        } else {
            string(profile, "font", "Monospace 12")
        };
        let foreground_background = foreground_background(profile);
        let bold = if boolean(profile, "bold-color-same-as-fg", true) {
            foreground_background.0
        } else {
            rgba(&string(profile, "bold-color", "#000000"))
        };
        let width = string(profile, "cjk-utf8-ambiguous-width", "narrow");
        Self {
            font: pango::FontDescription::from_string(&font_name),
            palette: palette(profile).iter().map(|item| rgba(item)).collect(),
            foreground: foreground_background.0,
            background: foreground_background.1,
            bold,
            bold_is_bright: boolean(profile, "bold-is-bright", false),
            cursor_colors: boolean(profile, "cursor-colors-set", false),
            cursor_background: rgba(&string(profile, "cursor-background-color", "#000000")),
            cursor_foreground: rgba(&string(profile, "cursor-foreground-color", "#ffffff")),
            highlight_colors: boolean(profile, "highlight-colors-set", false),
            highlight_background: rgba(&string(profile, "highlight-background-color", "#000000")),
            highlight_foreground: rgba(&string(profile, "highlight-foreground-color", "#ffffff")),
            scrollback_lines: int(profile, "scrollback-lines", 10000) as i64,
            scrollback_unlimited: boolean(profile, "scrollback-unlimited", false),
            scroll_on_keystroke: boolean(profile, "scroll-on-keystroke", true),
            scroll_on_output: boolean(profile, "scroll-on-output", false),
            scroll_on_insert: boolean(profile, "scroll-on-insert", true),
            audible_bell: boolean(profile, "audible-bell", true),
            cell_width_scale: double(profile, "cell-width-scale", 1.0),
            cell_height_scale: double(profile, "cell-height-scale", 1.0),
            enable_bidi: boolean(profile, "enable-bidi", true),
            enable_shaping: boolean(profile, "enable-shaping", true),
            enable_sixel: boolean(profile, "enable-sixel", false),
            cursor_blink: string(profile, "cursor-blink-mode", "system"),
            cursor_shape: string(profile, "cursor-shape", "block"),
            text_blink: string(profile, "text-blink-mode", "always"),
            backspace_binding: string(profile, "backspace-binding", "ascii-delete"),
            delete_binding: string(profile, "delete-binding", "delete-sequence"),
            cjk_ambiguous_width: if width == "wide" { 2 } else { 1 },
            scrollbar_policy: string(profile, "scrollbar-policy", "always"),
            login_shell: boolean(profile, "login-shell", false),
            use_custom_command: boolean(profile, "use-custom-command", false),
            custom_command: string(profile, "custom-command", ""),
            exit_action: string(profile, "exit-action", "close"),
            preserve_directory: string(profile, "preserve-working-directory", "safe"),
            word_char_exceptions: maybe_string(profile, "word-char-exceptions"),
            use_transparent_background: boolean(profile, "use-transparent-background", false),
            use_theme_transparency: boolean(profile, "use-theme-transparency", true),
            transparency_percent: int(profile, "background-transparency-percent", 50),
        }
    }
}

pub fn apply_terminal_look(terminal: &vte::Terminal, look: &TerminalLook) {
    terminal.set_font(Some(&look.font));
    let background = background_color(look);
    terminal.set_colors(Some(&look.foreground), Some(&background), &look.palette);
    terminal.set_color_bold(Some(&look.bold));
    terminal.set_bold_is_bright(look.bold_is_bright);
    if look.cursor_colors {
        terminal.set_color_cursor(Some(&look.cursor_background));
        terminal.set_color_cursor_foreground(Some(&look.cursor_foreground));
    } else {
        terminal.set_color_cursor(None);
        terminal.set_color_cursor_foreground(None);
    }
    if look.highlight_colors {
        terminal.set_color_highlight(Some(&look.highlight_background));
        terminal.set_color_highlight_foreground(Some(&look.highlight_foreground));
    } else {
        terminal.set_color_highlight(None);
        terminal.set_color_highlight_foreground(None);
    }
    if look.scrollback_unlimited {
        terminal.set_scrollback_lines(-1);
    } else {
        terminal.set_scrollback_lines(look.scrollback_lines);
    }
    terminal.set_scroll_on_keystroke(look.scroll_on_keystroke);
    terminal.set_scroll_on_output(look.scroll_on_output);
    terminal.set_scroll_on_insert(look.scroll_on_insert);
    terminal.set_audible_bell(look.audible_bell);
    terminal.set_cell_width_scale(look.cell_width_scale);
    terminal.set_cell_height_scale(look.cell_height_scale);
    terminal.set_enable_bidi(look.enable_bidi);
    terminal.set_enable_shaping(look.enable_shaping);
    terminal.set_enable_sixel(look.enable_sixel);
    terminal.set_cursor_blink_mode(match look.cursor_blink.as_str() {
        "on" => vte::CursorBlinkMode::On,
        "off" => vte::CursorBlinkMode::Off,
        _ => vte::CursorBlinkMode::System,
    });
    terminal.set_cursor_shape(match look.cursor_shape.as_str() {
        "ibeam" => vte::CursorShape::Ibeam,
        "underline" => vte::CursorShape::Underline,
        _ => vte::CursorShape::Block,
    });
    terminal.set_text_blink_mode(match look.text_blink.as_str() {
        "never" => vte::TextBlinkMode::Never,
        "focused" => vte::TextBlinkMode::Focused,
        "unfocused" => vte::TextBlinkMode::Unfocused,
        _ => vte::TextBlinkMode::Always,
    });
    terminal.set_backspace_binding(erase(&look.backspace_binding, vte::EraseBinding::AsciiDelete));
    terminal.set_delete_binding(erase(&look.delete_binding, vte::EraseBinding::DeleteSequence));
    terminal.set_cjk_ambiguous_width(look.cjk_ambiguous_width);
    terminal.set_word_char_exceptions(&look.word_char_exceptions);
}

pub fn scrollbar_policy(name: &str) -> (gtk::PolicyType, bool) {
    if name == "never" {
        (gtk::PolicyType::Never, false)
    } else if name == "overlay" {
        (gtk::PolicyType::Automatic, true)
    } else {
        (gtk::PolicyType::Always, false)
    }
}

pub fn terminal_theme_variant() -> String {
    terminal_settings()
        .and_then(|settings| {
            let value = settings.string("theme-variant").to_string();
            matches!(value.as_str(), "dark" | "light" | "system").then_some(value)
        })
        .unwrap_or_else(|| "system".to_string())
}

pub fn terminal_prefers_dark() -> bool {
    match terminal_theme_variant().as_str() {
        "dark" => true,
        "light" => false,
        _ => {
            gio::Settings::new("org.gnome.desktop.interface")
                .string("color-scheme")
                .as_str()
                == "prefer-dark"
        }
    }
}

pub fn apply_terminal_theme_preference() {
    if let Some(settings) = gtk::Settings::default() {
        settings.set_property("gtk-application-prefer-dark-theme", terminal_prefers_dark());
    }
}

pub fn gnome_profile_settings() -> Option<gio::Settings> {
    let profiles = gio::Settings::new("org.gnome.Terminal.ProfilesList");
    let default = profiles.string("default");
    if default.is_empty() {
        return None;
    }
    Some(gio::Settings::with_path(
        "org.gnome.Terminal.Legacy.Profile",
        &format!("/org/gnome/terminal/legacy/profiles:/{default}/"),
    ))
}

pub fn terminal_settings() -> Option<gio::Settings> {
    Some(gio::Settings::new("org.gnome.Terminal.Legacy.Settings"))
}

pub fn rgba_to_hex(color: &gdk::RGBA) -> String {
    let channel = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(color.red()),
        channel(color.green()),
        channel(color.blue())
    )
}

pub fn rgba(spec: &str) -> gdk::RGBA {
    gdk::RGBA::parse(spec).unwrap_or_else(|_| gdk::RGBA::parse("#000000").unwrap())
}

fn background_color(look: &TerminalLook) -> gdk::RGBA {
    let mut color = look.background;
    if look.use_transparent_background && !look.use_theme_transparency {
        color.set_alpha((1.0 - (look.transparency_percent as f64 / 100.0)).clamp(0.0, 1.0));
    } else {
        color.set_alpha(1.0);
    }
    color
}

fn foreground_background(profile: Option<&gio::Settings>) -> (gdk::RGBA, gdk::RGBA) {
    if !boolean(profile, "use-theme-colors", true) {
        return (
            rgba(&string(profile, "foreground-color", "#171421")),
            rgba(&string(profile, "background-color", "#ffffff")),
        );
    }
    apply_terminal_theme_preference();
    let label = gtk::Label::new(None);
    let foreground = label
        .style_context()
        .lookup_color("theme_fg_color")
        .unwrap_or_else(|| rgba(if terminal_prefers_dark() { "#f7f7f7" } else { "#3d3d3d" }));
    let background = label
        .style_context()
        .lookup_color("theme_base_color")
        .unwrap_or_else(|| rgba(if terminal_prefers_dark() { "#272727" } else { "#ffffff" }));
    (foreground, background)
}

fn palette(profile: Option<&gio::Settings>) -> Vec<String> {
    let Some(profile) = profile else {
        return FALLBACK_PALETTE.iter().map(|item| (*item).to_string()).collect();
    };
    let stored: Vec<String> = profile.strv("palette").iter().map(|item| item.to_string()).collect();
    if matches!(stored.len(), 8 | 16 | 232 | 256) {
        stored
    } else {
        FALLBACK_PALETTE.iter().map(|item| (*item).to_string()).collect()
    }
}

fn system_monospace_font() -> String {
    let name = gio::Settings::new("org.gnome.desktop.interface").string("monospace-font-name");
    if name.is_empty() { "Monospace 12".to_string() } else { name.to_string() }
}

fn maybe_string(profile: Option<&gio::Settings>, key: &str) -> String {
    let Some(profile) = profile else {
        return String::new();
    };
    profile.value(key).str().unwrap_or_default().to_string()
}

pub fn string(profile: Option<&gio::Settings>, key: &str, fallback: &str) -> String {
    profile
        .map(|item| item.string(key).to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

pub fn boolean(profile: Option<&gio::Settings>, key: &str, fallback: bool) -> bool {
    profile.map(|item| item.boolean(key)).unwrap_or(fallback)
}

fn double(profile: Option<&gio::Settings>, key: &str, fallback: f64) -> f64 {
    profile.map(|item| item.double(key)).unwrap_or(fallback)
}

pub fn int(profile: Option<&gio::Settings>, key: &str, fallback: i32) -> i32 {
    profile.map(|item| item.int(key)).unwrap_or(fallback)
}

fn erase(name: &str, fallback: vte::EraseBinding) -> vte::EraseBinding {
    match name {
        "auto" => vte::EraseBinding::Auto,
        "ascii-backspace" => vte::EraseBinding::AsciiBackspace,
        "ascii-delete" => vte::EraseBinding::AsciiDelete,
        "delete-sequence" => vte::EraseBinding::DeleteSequence,
        "tty" => vte::EraseBinding::Tty,
        _ => fallback,
    }
}
