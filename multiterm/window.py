"""The tabbed window.

A tab is one VTE widget, one history file, and the folder that process is in.
Tabs start in one plain notebook. A group exists only after the user creates
one, and then it is a tab above the terminal tabs. Closing the window writes
the session. Closing a tab drops it from the session and deletes its log.
"""

from __future__ import annotations

import contextlib
import os
import sys
import time
import uuid

import gi

gi.require_version("Gdk", "3.0")
gi.require_version("Gtk", "3.0")
gi.require_version("Vte", "2.91")

from gi.repository import Gdk, Gio, GLib, Gtk, Pango, Vte

from multiterm.launcher import ICON_NAME, icon_source
from multiterm.paths import data_dir, display_path, existing_directory, folder_uri, pid_cwd
from multiterm.preferences import PreferencesDialog
from multiterm.session import (
    GroupRecord,
    Session,
    TabRecord,
    gc_history,
    load_session,
    save_session,
)
from multiterm.shell import shell_init_path, tab_environment
from multiterm.style import (
    apply_terminal_look,
    apply_terminal_theme_preference,
    gnome_profile_settings,
    load_terminal_look,
    scrollbar_policy,
    terminal_settings,
)

_POLL_MS = 700
_SAVE_DELAY_MS = 400
_TAB_CSS = b"""
notebook tab { min-width: 180px; }
notebook.groups tab { min-width: 140px; }
"""
_DRAG_TARGET = "multiterm-tab"


class TabGroup:
    """Either the loose tabs or one group the user created.

    ``named`` is false for the loose notebook. That notebook is not shown as a
    group until a real group exists beside it.
    """

    def __init__(self, group_id: str, name: str, *, named: bool) -> None:
        self.id = group_id
        self.name = name
        self.named = named
        self.notebook = Gtk.Notebook()
        self.notebook.set_scrollable(True)
        self.notebook.set_show_border(False)
        self.notebook.multiterm_group = self
        self.label = Gtk.Label()
        self.label.set_halign(Gtk.Align.CENTER)
        self.label.set_margin_start(12)
        self.label.set_margin_end(12)
        self.label.set_max_width_chars(24)
        self.label.set_ellipsize(Pango.EllipsizeMode.END)
        self.tab_event = Gtk.EventBox()
        self.tab_event.add(self.label)
        self.refresh_label()

    def display_name(self) -> str:
        if not self.named:
            return "Ungrouped"
        return self.name.strip() or "Group"

    def refresh_label(self) -> None:
        text = self.display_name()
        self.label.set_text(text)
        self.label.set_tooltip_text(text)


class Tab:
    """One terminal page and the command log that belongs to it."""

    def __init__(self, tab_id: str, cwd: str) -> None:
        self.id = tab_id
        self.cwd = existing_directory(cwd)
        self.title = ""
        self.group: TabGroup | None = None
        self.history_path = data_dir() / "history" / tab_id
        self.pid: int | None = None
        self.spawned = False
        self.closing = False
        self.widget: Gtk.Widget | None = None
        self.terminal = Vte.Terminal()
        self.terminal.tab = self
        self.label = Gtk.Label(label=display_path(self.cwd))
        self.label.set_halign(Gtk.Align.START)
        self.label.set_xalign(0)
        self.label.set_max_width_chars(64)
        self.label.set_ellipsize(Pango.EllipsizeMode.MIDDLE)
        self.label.set_margin_start(8)
        self.label.set_margin_end(8)

    def prepare_history_file(self) -> None:
        self.history_path.parent.mkdir(parents=True, exist_ok=True)
        self.history_path.touch(exist_ok=True)
        with contextlib.suppress(OSError):
            os.chmod(self.history_path, 0o600)

    @property
    def ready_path(self):
        return self.history_path.with_name(self.history_path.name + ".ready")

    def display_name(self) -> str:
        title = self.title.strip()
        if title:
            return title
        return display_path(self.cwd)

    def tooltip_text(self) -> str:
        path = display_path(self.cwd)
        title = self.title.strip()
        if title and title != path:
            return f"{title}\n{path}"
        return path


class MainWindow(Gtk.ApplicationWindow):
    """Single window. Reopening the app restores the tabs left open."""

    def __init__(self, app) -> None:
        super().__init__(application=app, title="Multiterm")
        self.app = app
        self.quitting = False
        self._save_id = 0
        self._look = load_terminal_look()
        self._preferences: PreferencesDialog | None = None
        self._profile_settings = None
        self._profile_handler = 0
        self._terminal_settings = None
        self._active_group: TabGroup | None = None
        self.groups: list[TabGroup] = []
        self._drag_targets = [
            Gtk.TargetEntry.new(_DRAG_TARGET, Gtk.TargetFlags.SAME_APP, 0),
        ]
        self._apply_icon()
        self.set_default_size(1000, 640)
        self.header = self._build_header()
        self.set_titlebar(self.header)
        self.groups_notebook = Gtk.Notebook()
        self.groups_notebook.set_show_tabs(False)
        self.groups_notebook.set_show_border(False)
        self.groups_notebook.set_scrollable(True)
        self.groups_notebook.get_style_context().add_class("groups")
        self.groups_notebook.connect("switch-page", self._on_group_switched)
        self.add(self.groups_notebook)
        self._install_tab_css()
        self._bind_actions()
        self.connect("delete-event", self.on_delete)
        self.connect("configure-event", self.on_configure)
        self.connect("show", lambda *_args: self._sync_group_tabs())
        self._restore_or_start()
        self._watch_terminal_settings()
        GLib.timeout_add(_POLL_MS, self._poll_directories)

    def show_all(self) -> None:
        super().show_all()
        self._sync_group_tabs()

    def _apply_icon(self) -> None:
        path = icon_source()
        if path.is_file() and self.set_icon_from_file(str(path)):
            with contextlib.suppress(GLib.Error):
                Gtk.Window.set_default_icon_from_file(str(path))
            return
        self.set_icon_name(ICON_NAME)

    def tabs(self) -> list[Tab]:
        found: list[Tab] = []
        for group in self.groups:
            found.extend(self._group_tabs(group))
        return found

    def current_group(self) -> TabGroup | None:
        if not self.groups:
            return None
        index = self.groups_notebook.get_current_page()
        if index >= 0:
            group = getattr(self.groups_notebook.get_nth_page(index), "multiterm_group", None)
            if isinstance(group, TabGroup):
                return group
        return self.groups[0]

    def current_tab(self) -> Tab | None:
        group = self.current_group()
        if group is None:
            return None
        index = group.notebook.get_current_page()
        if index < 0:
            return None
        tab = getattr(group.notebook.get_nth_page(index), "tab", None)
        return tab if isinstance(tab, Tab) else None

    def active_group(self) -> TabGroup:
        group = self.current_group()
        if group is not None:
            return group
        group = self.add_group("", named=False)
        self._show_group(group)
        return group

    def loose_group(self) -> TabGroup:
        for group in self.groups:
            if not group.named:
                return group
        group = self.add_group("", named=False)
        self._move_group(group, 0)
        return group

    def add_group(
        self,
        name: str = "",
        group_id: str | None = None,
        *,
        named: bool = False,
    ) -> TabGroup:
        group = TabGroup(group_id or uuid.uuid4().hex, name, named=named)
        group.notebook.connect("switch-page", self._on_switch_page)
        group.notebook.connect("button-press-event", self._on_notebook_button)
        group.notebook.connect("page-reordered", lambda *_args: self.schedule_save())
        group.notebook.connect("page-added", self._on_page_added)
        group.tab_event.connect(
            "button-press-event",
            lambda _widget, event, item=group: self._on_group_tab_button(item, event),
        )
        self._enable_group_drop(group)
        self.groups.append(group)
        self.groups_notebook.append_page(group.notebook, group.tab_event)
        self.groups_notebook.set_tab_reorderable(group.notebook, False)
        self.groups_notebook.set_tab_detachable(group.notebook, False)
        group.notebook.show_all()
        group.tab_event.show_all()
        self._sync_group_tabs()
        self.schedule_save()
        return group

    def new_group(self, tab: Tab | None = None) -> TabGroup:
        """Create Group N and move the current tab into it."""
        if not isinstance(tab, Tab):
            tab = self.current_tab()
        group = self.add_group(self._next_group_name(), named=True)
        if tab is not None:
            self.move_tab_to_group(tab, group)
        self._show_group(group)
        return group

    def set_group_name(self, group: TabGroup, name: str) -> None:
        cleaned = name.strip()
        if not group.named or not cleaned:
            return
        group.name = cleaned
        group.refresh_label()
        self.schedule_save()

    def rename_group(self, group: TabGroup) -> None:
        if not group.named:
            return
        dialog = Gtk.Dialog(title="Rename group", transient_for=self, modal=True)
        dialog.add_button("Cancel", Gtk.ResponseType.CANCEL)
        dialog.add_button("Rename", Gtk.ResponseType.OK)
        dialog.set_default_response(Gtk.ResponseType.OK)
        entry = Gtk.Entry()
        entry.set_text(group.name)
        entry.set_activates_default(True)
        area = dialog.get_content_area()
        area.set_spacing(8)
        area.set_margin_start(12)
        area.set_margin_end(12)
        area.set_margin_top(12)
        area.set_margin_bottom(12)
        area.add(entry)
        dialog.set_default_size(360, -1)
        dialog.show_all()
        response = dialog.run()
        title = entry.get_text()
        dialog.destroy()
        if response == Gtk.ResponseType.OK:
            self.set_group_name(group, title)

    def ungroup(self, group: TabGroup) -> None:
        """Move the group's tabs back with the loose tabs and remove the group."""
        if not group.named or group not in self.groups:
            return
        loose = self.loose_group()
        for tab in self._group_tabs(group):
            self.move_tab_to_group(tab, loose, focus=False)
        self._remove_group(group)
        self._show_group(loose)

    def close_group_terminals(self, group: TabGroup) -> None:
        for tab in self._group_tabs(group):
            if self.quitting:
                return
            self.close_tab(tab)
            if self.quitting:
                return
        if group in self.groups and group.named and group.notebook.get_n_pages() == 0:
            self._remove_group(group)

    def add_tab(
        self,
        cwd: str,
        tab_id: str | None = None,
        *,
        title: str = "",
        group: TabGroup | None = None,
        focus: bool = True,
    ) -> Tab:
        if group is None:
            group = self.active_group()
        tab = Tab(tab_id or uuid.uuid4().hex, cwd)
        tab.title = title.strip()
        tab.group = group
        tab.prepare_history_file()
        self._configure_terminal(tab)
        page = self._wrap_terminal(tab)
        header = self._tab_header(tab)
        group.notebook.insert_page(page, header, self._insert_position(group, focus))
        group.notebook.set_tab_reorderable(page, True)
        group.notebook.set_tab_detachable(page, False)
        page.show_all()
        if focus:
            self._show_group(group)
            group.notebook.set_current_page(group.notebook.page_num(page))
            tab.terminal.grab_focus()
        if tab.terminal.get_realized():
            self.spawn_tab(tab)
        self.refresh_labels()
        self.schedule_save()
        return tab

    def new_tab(self) -> None:
        current = self.current_tab()
        if current is None and self.tabs():
            current = self.tabs()[-1]
        if current is None or self._look.preserve_directory == "never":
            cwd = str(os.path.expanduser("~"))
        else:
            cwd = current.cwd
        self.add_tab(cwd, focus=True)

    def close_current_tab(self) -> None:
        tab = self.current_tab()
        if tab is not None:
            self.close_tab(tab)

    def close_tab(self, tab: Tab) -> bool:
        if self.quitting or tab.closing:
            return False
        tab.closing = True
        group = tab.group
        if tab.widget is not None and group is not None:
            index = group.notebook.page_num(tab.widget)
            if index >= 0:
                group.notebook.remove_page(index)
        self.save()
        tab.history_path.unlink(missing_ok=True)
        tab.ready_path.unlink(missing_ok=True)
        if not self.tabs():
            self.close()
            return False
        self.refresh_labels()
        self.refresh_header()
        return False

    def focus_tab(self, index: int) -> None:
        group = self.current_group()
        if group is None or index < 0 or index >= group.notebook.get_n_pages():
            return
        group.notebook.set_current_page(index)
        tab = self.current_tab()
        if tab is not None:
            tab.terminal.grab_focus()

    def previous_tab(self) -> None:
        group = self.current_group()
        if group is not None:
            group.notebook.prev_page()

    def next_tab(self) -> None:
        group = self.current_group()
        if group is not None:
            group.notebook.next_page()

    def set_tab_title(self, tab: Tab, title: str) -> None:
        tab.title = title.strip()
        self.refresh_labels()
        self.refresh_header()
        self.schedule_save()

    def rename_tab(self, tab: Tab) -> None:
        dialog = Gtk.Dialog(title="Rename tab", transient_for=self, modal=True)
        dialog.add_button("Cancel", Gtk.ResponseType.CANCEL)
        dialog.add_button("Rename", Gtk.ResponseType.OK)
        dialog.set_default_response(Gtk.ResponseType.OK)
        entry = Gtk.Entry()
        entry.set_text(tab.title)
        entry.set_activates_default(True)
        entry.set_placeholder_text(display_path(tab.cwd))
        area = dialog.get_content_area()
        area.set_spacing(8)
        area.set_margin_start(12)
        area.set_margin_end(12)
        area.set_margin_top(12)
        area.set_margin_bottom(12)
        note = Gtk.Label(label="Leave the name empty to show the folder.")
        note.set_halign(Gtk.Align.START)
        area.add(note)
        area.add(entry)
        dialog.set_default_size(360, -1)
        dialog.show_all()
        response = dialog.run()
        title = entry.get_text()
        dialog.destroy()
        if response == Gtk.ResponseType.OK:
            self.set_tab_title(tab, title)

    def move_tab_to_group(self, tab: Tab, group: TabGroup, *, focus: bool = True) -> None:
        page = tab.widget
        source = tab.group
        if page is None or source is None or source is group or group not in self.groups:
            return
        label = source.notebook.get_tab_label(page)
        if label is None:
            return
        # Hold the Python wrappers across remove_page. PyGObject's toggle
        # references keep the widgets alive; GObject.ref() is unsupported.
        index = source.notebook.page_num(page)
        if index >= 0:
            source.notebook.remove_page(index)
        destination = group.notebook.append_page(page, label)
        group.notebook.set_tab_reorderable(page, True)
        group.notebook.set_tab_detachable(page, False)
        page.show_all()
        tab.group = group
        if focus:
            self._show_group(group)
            group.notebook.set_current_page(destination)
            tab.terminal.grab_focus()
        self.schedule_save()

    def copy_current(self) -> None:
        tab = self.current_tab()
        if tab is not None:
            tab.terminal.copy_clipboard_format(Vte.Format.TEXT)

    def paste_current(self) -> None:
        tab = self.current_tab()
        if tab is not None:
            tab.terminal.paste_clipboard()

    def open_current_folder(self) -> None:
        tab = self.current_tab()
        if tab is None:
            return
        self.open_folder(pid_cwd(tab.pid) or tab.cwd)

    def open_folder(self, path: str) -> None:
        path = existing_directory(path)
        try:
            Gio.AppInfo.launch_default_for_uri(folder_uri(path), None)
        except GLib.Error as error:
            dialog = Gtk.MessageDialog(
                transient_for=self,
                modal=True,
                message_type=Gtk.MessageType.ERROR,
                buttons=Gtk.ButtonsType.CLOSE,
                text="Could not open the folder",
            )
            dialog.format_secondary_text(error.message)
            dialog.run()
            dialog.destroy()

    def open_preferences(self) -> None:
        if self._preferences is not None:
            self._preferences.present()
            return
        dialog = PreferencesDialog(self)
        dialog.connect("destroy", self._on_preferences_closed)
        self._preferences = dialog
        dialog.show_all()

    def refresh_labels(self) -> None:
        for tab in self.tabs():
            tab.label.set_text(tab.display_name())
            tab.label.set_tooltip_text(tab.tooltip_text())

    def refresh_header(self) -> bool:
        tab = self.current_tab()
        if tab is None:
            self.header.set_subtitle("")
            self.set_title("Multiterm")
            self.open_button.set_sensitive(False)
            return False
        path = display_path(tab.cwd)
        self.header.set_subtitle(path)
        self.set_title(f"{tab.display_name()} — Multiterm")
        self.open_button.set_sensitive(os.path.isdir(tab.cwd))
        return False

    def schedule_save(self) -> None:
        if self.quitting or self._save_id:
            return
        self._save_id = GLib.timeout_add(_SAVE_DELAY_MS, self._flush_save)

    def save(self) -> None:
        width, height = self.get_size()
        groups: list[GroupRecord] = []
        active_group = 0
        current = self.current_group()
        for index, group in enumerate(self.groups):
            if group is current:
                active_group = index
            groups.append(
                GroupRecord(
                    id=group.id,
                    name=group.name if group.named else "",
                    named=group.named,
                    active=max(group.notebook.get_current_page(), 0),
                    tabs=[
                        TabRecord(id=tab.id, cwd=tab.cwd, title=tab.title)
                        for tab in self._group_tabs(group)
                    ],
                )
            )
        save_session(
            Session(
                groups=groups,
                active_group=active_group,
                width=int(width),
                height=int(height),
            )
        )

    def spawn_tab(self, tab: Tab) -> None:
        if tab.spawned or tab.closing:
            return
        tab.spawned = True
        tab.cwd = existing_directory(tab.cwd)
        tab.terminal.spawn_async(
            Vte.PtyFlags.DEFAULT,
            tab.cwd,
            self._shell_argv(),
            tab_environment(tab.history_path),
            GLib.SpawnFlags.DEFAULT,
            None,
            None,
            -1,
            None,
            self._on_spawned,
        )

    def on_delete(self, *_args) -> bool:
        self.quitting = True
        self.save()
        return False

    def on_configure(self, _widget, _event) -> bool:
        self.schedule_save()
        return False

    def reload_appearance(self, *_args) -> None:
        """Re-read the GNOME Terminal profile and apply it to every open tab."""
        apply_terminal_theme_preference()
        self._look = load_terminal_look()
        for tab in self.tabs():
            apply_terminal_look(tab.terminal, self._look)
            self._apply_scrollbar(tab)

    def _build_header(self) -> Gtk.HeaderBar:
        header = Gtk.HeaderBar()
        header.set_show_close_button(True)
        header.set_title("Multiterm")
        new_button = Gtk.Button.new_from_icon_name("tab-new-symbolic", Gtk.IconSize.BUTTON)
        new_button.set_tooltip_text("New tab (Ctrl+Shift+T)")
        new_button.connect("clicked", lambda *_args: self.new_tab())
        header.pack_start(new_button)
        self.open_button = Gtk.Button.new_from_icon_name(
            "folder-open-symbolic",
            Gtk.IconSize.BUTTON,
        )
        self.open_button.set_tooltip_text("Open folder in the file manager (Ctrl+Shift+O)")
        self.open_button.connect("clicked", lambda *_args: self.open_current_folder())
        header.pack_start(self.open_button)
        group_button = Gtk.Button.new_from_icon_name("list-add-symbolic", Gtk.IconSize.BUTTON)
        group_button.set_tooltip_text("New group (Ctrl+Shift+G)")
        group_button.connect("clicked", lambda *_args: self.new_group())
        header.pack_start(group_button)
        preferences = Gtk.Button.new_from_icon_name(
            "preferences-system-symbolic",
            Gtk.IconSize.BUTTON,
        )
        preferences.set_tooltip_text("Preferences")
        preferences.connect("clicked", lambda *_args: self.open_preferences())
        header.pack_end(preferences)
        return header

    def _install_tab_css(self) -> None:
        css = Gtk.CssProvider()
        css.load_from_data(_TAB_CSS)
        screen = Gdk.Screen.get_default()
        if screen is None:
            return
        Gtk.StyleContext.add_provider_for_screen(
            screen,
            css,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
        )

    def _bind_actions(self) -> None:
        self._add_action("new-tab", self.new_tab, ["<Shift><Control>t"])
        self._add_action("close-tab", self.close_current_tab, ["<Shift><Control>w"])
        self._add_action("open-folder", self.open_current_folder, ["<Shift><Control>o"])
        self._add_action("new-group", self.new_group, ["<Shift><Control>g"])
        self._add_action("preferences", self.open_preferences, [])
        self._add_action("copy", self.copy_current, ["<Shift><Control>c"])
        self._add_action("paste", self.paste_current, ["<Shift><Control>v"])
        self._add_action("quit", self.close, ["<Shift><Control>q"])
        self._add_action("prev-tab", self.previous_tab, ["<Control>Page_Up"])
        self._add_action("next-tab", self.next_tab, ["<Control>Page_Down"])
        for index in range(1, 10):
            self._add_action(
                f"tab-{index}",
                lambda idx=index - 1: self.focus_tab(idx),
                [f"<Alt>{index}"],
            )

    def _add_action(self, name: str, callback, accels: list[str]) -> None:
        action = Gio.SimpleAction.new(name, None)
        action.connect("activate", lambda *_args: callback())
        self.add_action(action)
        self.app.set_accels_for_action(f"win.{name}", accels)

    def _restore_or_start(self) -> None:
        session = load_session()
        keep: set[str] = set()
        active_index = 0
        if session is not None:
            size = session.window_size()
            if size is not None:
                self.set_default_size(*size)
            for record in session.groups:
                group = self.add_group(record.name, group_id=record.id, named=record.named)
                for tab_record in record.tabs:
                    keep.add(tab_record.id)
                    self.add_tab(
                        tab_record.cwd,
                        tab_id=tab_record.id,
                        title=tab_record.title,
                        group=group,
                        focus=False,
                    )
                pages = group.notebook.get_n_pages()
                if pages:
                    active = record.active if 0 <= record.active < pages else 0
                    group.notebook.set_current_page(active)
            active_index = session.active_group
            if self.groups and all(group.named for group in self.groups):
                loose = self.add_group("", named=False)
                self._move_group(loose, 0)
                active_index += 1
        if not self.groups:
            self.add_group("", named=False)
        if not self.tabs():
            tab = self.add_tab(str(os.path.expanduser("~")), focus=True)
            keep = {tab.id}
        elif self.groups:
            if active_index < 0 or active_index >= len(self.groups):
                active_index = 0
            self._show_group(self.groups[active_index])
        gc_history(keep)
        self.refresh_labels()
        self.refresh_header()
        self._sync_group_tabs()

    def _configure_terminal(self, tab: Tab) -> None:
        terminal = tab.terminal
        apply_terminal_look(terminal, self._look)
        terminal.set_hexpand(True)
        terminal.set_vexpand(True)
        terminal.connect("child-exited", self._on_child_exited)
        terminal.connect("button-press-event", self._on_terminal_button)
        terminal.connect("realize", self._on_terminal_realize)

    def _wrap_terminal(self, tab: Tab) -> Gtk.ScrolledWindow:
        page = Gtk.ScrolledWindow()
        self._apply_scrollbar(tab, page)
        page.add(tab.terminal)
        page.tab = tab
        tab.widget = page
        return page

    def _apply_scrollbar(self, tab: Tab, page: Gtk.ScrolledWindow | None = None) -> None:
        widget = page if page is not None else tab.widget
        if not isinstance(widget, Gtk.ScrolledWindow):
            return
        vpolicy, overlay = scrollbar_policy(self._look.scrollbar_policy)
        widget.set_policy(Gtk.PolicyType.NEVER, vpolicy)
        widget.set_overlay_scrolling(overlay)

    def _tab_header(self, tab: Tab) -> Gtk.EventBox:
        header = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=4)
        header.pack_start(tab.label, True, True, 0)
        close = Gtk.Button.new_from_icon_name("window-close-symbolic", Gtk.IconSize.MENU)
        close.set_relief(Gtk.ReliefStyle.NONE)
        close.set_tooltip_text("Close tab")
        close.connect("clicked", lambda *_args, target=tab: self.close_tab(target))
        header.pack_start(close, False, False, 0)
        event = Gtk.EventBox()
        event.add(header)
        event.connect(
            "button-press-event",
            lambda _widget, button_event, target=tab: self._on_tab_label_button(
                target,
                button_event,
            ),
        )
        event.drag_source_set(
            Gdk.ModifierType.BUTTON1_MASK,
            self._drag_targets,
            Gdk.DragAction.MOVE,
        )
        event.connect("drag-data-get", self._on_tab_drag_get, tab)
        event.show_all()
        return event

    def _enable_group_drop(self, group: TabGroup) -> None:
        group.tab_event.drag_dest_set(
            Gtk.DestDefaults.HIGHLIGHT | Gtk.DestDefaults.MOTION | Gtk.DestDefaults.DROP,
            self._drag_targets,
            Gdk.DragAction.MOVE,
        )
        group.tab_event.connect("drag-data-received", self._on_group_drop, group)

    def _on_tab_drag_get(self, _widget, _context, data, _info, _time, tab: Tab) -> None:
        data.set_text(tab.id, -1)

    def _on_group_drop(self, _widget, context, _x, _y, data, _info, time, group: TabGroup) -> None:
        tab_id = data.get_text()
        tab = self._tab_by_id(tab_id) if tab_id else None
        if tab is not None:
            self.move_tab_to_group(tab, group)
        context.finish(tab is not None, False, time)

    def _on_terminal_realize(self, terminal: Vte.Terminal) -> None:
        tab = terminal.tab
        if isinstance(tab, Tab) and not tab.spawned:
            self.spawn_tab(tab)

    def _on_spawned(self, terminal: Vte.Terminal, pid, error, *_) -> None:
        tab = terminal.tab
        if not isinstance(tab, Tab):
            return
        if error is not None:
            message = getattr(error, "message", str(error))
            print(f"Could not start the shell: {message}", file=sys.stderr)
            return
        try:
            tab.pid = int(pid)
        except (TypeError, ValueError):
            tab.pid = None

    def _on_child_exited(self, terminal: Vte.Terminal, _status) -> None:
        tab = terminal.tab
        if self.quitting or not isinstance(tab, Tab) or tab.closing:
            return
        action = self._look.exit_action
        if action == "hold":
            return
        if action == "restart":
            tab.spawned = False
            GLib.idle_add(self.spawn_tab, tab)
            return
        GLib.idle_add(self.close_tab, tab)

    def _on_terminal_button(self, terminal: Vte.Terminal, event) -> bool:
        if event.button != 3 or not isinstance(terminal.tab, Tab):
            return False
        self._popup_tab_menu(terminal.tab, event)
        return True

    def _on_tab_label_button(self, tab: Tab, event) -> bool:
        if event.type == Gdk.EventType._2BUTTON_PRESS and event.button == 1:
            self.rename_tab(tab)
            return True
        if event.button == 3:
            self._popup_tab_menu(tab, event)
            return True
        return False

    def _on_notebook_button(self, notebook, event) -> bool:
        """Rename on double-click, and the tab menu on right-click, without eating drags."""
        tab = self._tab_at(notebook, event.x, event.y)
        if tab is None:
            return False
        return self._on_tab_label_button(tab, event)

    def _on_group_tab_button(self, group: TabGroup, event) -> bool:
        if not group.named:
            return False
        if event.type == Gdk.EventType._2BUTTON_PRESS and event.button == 1:
            self.rename_group(group)
            return True
        if event.button == 3:
            self._popup_group_menu(group, event)
            return True
        return False

    def _tab_at(self, notebook: Gtk.Notebook, x: float, y: float) -> Tab | None:
        for index in range(notebook.get_n_pages()):
            page = notebook.get_nth_page(index)
            label = notebook.get_tab_label(page)
            if label is None:
                continue
            origin = label.translate_coordinates(notebook, 0, 0)
            if origin is None:
                continue
            left, top = origin
            width = label.get_allocated_width()
            height = label.get_allocated_height()
            if left <= x <= left + width and top <= y <= top + height:
                tab = getattr(page, "tab", None)
                if isinstance(tab, Tab):
                    return tab
        return None

    def _popup_tab_menu(self, tab: Tab, event) -> None:
        menu = Gtk.Menu()
        actions: list[tuple[str, object]] = [
            ("Rename", lambda: self.rename_tab(tab)),
            ("New Group", lambda: self.new_group(tab)),
        ]
        if any(group.named for group in self.groups):
            for group in self.groups:
                if group is tab.group:
                    continue
                actions.append(
                    (
                        f"Move to {group.display_name()}",
                        lambda item=group: self.move_tab_to_group(tab, item),
                    )
                )
        actions.extend(
            (
                ("Copy", lambda: tab.terminal.copy_clipboard_format(Vte.Format.TEXT)),
                ("Paste", tab.terminal.paste_clipboard),
                ("Open Folder", lambda: self.open_folder(pid_cwd(tab.pid) or tab.cwd)),
                ("Close Tab", lambda: self.close_tab(tab)),
            )
        )
        for label, callback in actions:
            item = Gtk.MenuItem(label=label)
            item.connect("activate", lambda *_item, func=callback: func())
            menu.append(item)
        menu.show_all()
        menu.popup_at_pointer(event)

    def _popup_group_menu(self, group: TabGroup, event) -> None:
        menu = Gtk.Menu()
        actions = (
            ("Rename", lambda: self.rename_group(group)),
            ("Ungroup", lambda: self.ungroup(group)),
            ("Close All Terminals", lambda: self.close_group_terminals(group)),
        )
        for label, callback in actions:
            item = Gtk.MenuItem(label=label)
            item.connect("activate", lambda *_item, func=callback: func())
            menu.append(item)
        menu.show_all()
        menu.popup_at_pointer(event)

    def _on_switch_page(self, notebook, page, _index) -> None:
        group = getattr(notebook, "multiterm_group", None)
        if isinstance(group, TabGroup):
            self._active_group = group

        def focus() -> bool:
            tab = getattr(page, "tab", None)
            if isinstance(tab, Tab):
                tab.terminal.grab_focus()
            else:
                page.grab_focus()
            return False

        GLib.idle_add(self.refresh_header)
        GLib.idle_add(focus)
        self.schedule_save()

    def _on_group_switched(self, _notebook, page, _index) -> None:
        group = getattr(page, "multiterm_group", None)
        if isinstance(group, TabGroup):
            self._active_group = group
        GLib.idle_add(self.refresh_header)
        self.schedule_save()

    def _on_page_added(self, notebook, page, _index) -> None:
        group = getattr(notebook, "multiterm_group", None)
        tab = getattr(page, "tab", None)
        if isinstance(tab, Tab) and isinstance(group, TabGroup):
            tab.group = group
            notebook.set_tab_reorderable(page, True)
            notebook.set_tab_detachable(page, False)
        self.schedule_save()

    def _show_group(self, group: TabGroup) -> None:
        index = self.groups_notebook.page_num(group.notebook)
        if index >= 0:
            self.groups_notebook.set_current_page(index)
        self._active_group = group

    def _move_group(self, group: TabGroup, position: int) -> None:
        if group not in self.groups:
            return
        self.groups.remove(group)
        self.groups.insert(position, group)
        self.groups_notebook.reorder_child(group.notebook, position)

    def _remove_group(self, group: TabGroup) -> None:
        if group not in self.groups or not group.named:
            return
        self.groups.remove(group)
        index = self.groups_notebook.page_num(group.notebook)
        if index >= 0:
            self.groups_notebook.remove_page(index)
        if self._active_group is group or self.current_group() is None:
            loose = self.loose_group()
            self._show_group(loose)
        self._sync_group_tabs()
        self.schedule_save()

    def _sync_group_tabs(self) -> None:
        self.groups_notebook.set_show_tabs(any(group.named for group in self.groups))

    def _group_tabs(self, group: TabGroup) -> list[Tab]:
        found: list[Tab] = []
        for index in range(group.notebook.get_n_pages()):
            tab = getattr(group.notebook.get_nth_page(index), "tab", None)
            if isinstance(tab, Tab):
                found.append(tab)
        return found

    def _tab_by_id(self, tab_id: str) -> Tab | None:
        for tab in self.tabs():
            if tab.id == tab_id:
                return tab
        return None

    def _next_group_name(self) -> str:
        existing = {group.name for group in self.groups if group.named}
        index = 1
        while f"Group {index}" in existing:
            index += 1
        return f"Group {index}"

    def _insert_position(self, group: TabGroup, focus: bool) -> int:
        count = group.notebook.get_n_pages()
        if not focus or self._new_tab_position() == "last":
            return count
        return max(group.notebook.get_current_page() + 1, 0)

    def _new_tab_position(self) -> str:
        settings = self._terminal_settings
        if settings is None:
            return "last"
        try:
            return settings.get_string("new-tab-position") or "last"
        except GLib.Error:
            return "last"

    def _shell_argv(self) -> list[str]:
        command = self._look.custom_command.strip()
        if self._look.use_custom_command and command:
            argv = _parsed_command(command)
            if argv:
                return argv
        if self._look.login_shell:
            return ["/bin/bash", "--login", "--rcfile", str(shell_init_path())]
        return ["/bin/bash", "--rcfile", str(shell_init_path())]

    def _watch_terminal_settings(self) -> None:
        self._terminal_settings = terminal_settings()
        if self._terminal_settings is not None:
            self._terminal_settings.connect("changed", self.reload_appearance)
        try:
            profiles = Gio.Settings.new("org.gnome.Terminal.ProfilesList")
        except GLib.Error:
            profiles = None
        if profiles is not None:
            profiles.connect("changed::default", self._on_default_profile_changed)
        self._bind_profile_settings(gnome_profile_settings())

    def _on_default_profile_changed(self, *_args) -> None:
        self._bind_profile_settings(gnome_profile_settings())
        self.reload_appearance()

    def _bind_profile_settings(self, settings) -> None:
        if self._profile_settings is not None and self._profile_handler:
            with contextlib.suppress(TypeError):
                self._profile_settings.disconnect(self._profile_handler)
            self._profile_handler = 0
        self._profile_settings = settings
        if settings is not None:
            self._profile_handler = settings.connect("changed", self.reload_appearance)

    def _on_preferences_closed(self, *_args) -> None:
        self._preferences = None

    def _poll_directories(self) -> bool:
        """Follow ``cd`` by reading the shell's directory from /proc."""
        if self.quitting:
            return False
        changed = False
        for tab in self.tabs():
            cwd = pid_cwd(tab.pid)
            if cwd and cwd != tab.cwd:
                tab.cwd = cwd
                changed = True
        if changed:
            self.refresh_labels()
            self.refresh_header()
            self.schedule_save()
        return True

    def continue_bench(self) -> bool:
        """Open the requested tabs and time the shell and a typed command."""
        wanted = int(os.environ.get("MULTITERM_BENCH_TABS", "1"))
        while len(self.tabs()) < wanted:
            self.new_tab()
        started = time.monotonic()
        GLib.timeout_add(50, self._bench_shells, started, wanted)
        return False

    def _bench_shells(self, started: float, wanted: int) -> bool:
        tabs = self.tabs()[:wanted]
        if not all(tab.ready_path.exists() for tab in tabs):
            if time.monotonic() - started > 20:
                print('BENCH metrics {"shells_ms":-1,"echo_ms":-1,"tabs":0}', flush=True)
                return False
            return True
        shells_ms = int((time.monotonic() - started) * 1000)
        for tab in tabs:
            tab.terminal.feed_child(b"printf 'multiterm-echo-marker\\n'\n")
        GLib.timeout_add(20, self._bench_echo, shells_ms, time.monotonic(), wanted)
        return False

    def _bench_echo(self, shells_ms: int, started: float, wanted: int) -> bool:
        tabs = self.tabs()[:wanted]
        if not all(_terminal_has_marker(tab) for tab in tabs):
            if time.monotonic() - started < 10:
                return True
            echo_ms = -1
        else:
            echo_ms = int((time.monotonic() - started) * 1000)
        print(
            f'BENCH metrics {{"shells_ms":{shells_ms},"echo_ms":{echo_ms},"tabs":{len(tabs)}}}',
            flush=True,
        )
        return False

    def _flush_save(self) -> bool:
        self._save_id = 0
        if not self.quitting:
            self.save()
        return False


def _terminal_has_marker(tab: Tab) -> bool:
    text = tab.terminal.get_text_format(Vte.Format.TEXT)
    if isinstance(text, tuple):
        text = text[0]
    return text is not None and "multiterm-echo-marker" in str(text)


def _parsed_command(command: str) -> list[str] | None:
    try:
        parsed = GLib.shell_parse_argv(command)
    except GLib.Error:
        return None
    if isinstance(parsed, tuple):
        if not parsed or not parsed[0]:
            return None
        values = parsed[1] if len(parsed) > 1 else None
    else:
        values = parsed
    if not values:
        return None
    return [str(item) for item in values]
