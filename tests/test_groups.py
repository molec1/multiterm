"""Rename a tab and move it into a named group. Needs a display."""

from __future__ import annotations

import json
import os

import pytest

gi = pytest.importorskip("gi")
gi.require_version("Gtk", "3.0")
from gi.repository import GLib
from multiterm.app import MultitermApp
from multiterm.paths import data_dir

pytestmark = pytest.mark.skipif(not os.environ.get("DISPLAY"), reason="needs a display")


def test_rename_and_group_are_saved(tmp_path, monkeypatch):
    monkeypatch.setenv("MULTITERM_DATA", str(tmp_path))
    error: dict[str, str] = {}

    def on_ready(window) -> None:
        try:
            _rename_and_group(window)
        except Exception as exc:
            error["message"] = f"{type(exc).__name__}: {exc}"
            window.app.exit_code = 1
        else:
            window.app.exit_code = 0
        window.quitting = True
        GLib.idle_add(window.app.quit)

    app = MultitermApp(testing=True, on_ready=on_ready)
    app.run(["multiterm"])
    assert error.get("message") is None, error.get("message")
    assert app.exit_code == 0


def _rename_and_group(window) -> None:
    window.set_keep_below(True)
    window.set_skip_taskbar_hint(True)
    window.move(0, 4000)
    tab = window.current_tab()
    assert tab is not None
    window.set_tab_title(tab, "api")
    assert tab.label.get_text() == "api"
    assert tab.display_name() == "api"
    assert window.groups_notebook.get_show_tabs() is False
    extra = window.add_tab(tab.cwd, focus=False)
    group = window.new_group()
    assert group.name == "Group 1"
    assert group.named is True
    assert window.groups_notebook.get_show_tabs() is True
    assert group.tab_event.drag_dest_get_target_list() is not None
    assert extra.group is not group
    assert extra.group.named is False
    assert tab.group is group
    window.set_group_name(group, "Work")
    assert group.label.get_text() == "Work"
    while GLib.main_context_default().pending():
        GLib.main_context_default().iteration(False)
    window.save()
    stored = json.loads((data_dir() / "session.json").read_text(encoding="utf-8"))
    by_name = {item["name"]: item for item in stored["groups"]}
    assert by_name[""]["named"] is False
    assert [page["title"] for page in by_name[""]["tabs"]] == [""]
    assert by_name["Work"]["named"] is True
    assert [page["title"] for page in by_name["Work"]["tabs"]] == ["api"]
    window.ungroup(group)
    assert window.groups_notebook.get_show_tabs() is False
    assert tab.group is extra.group
    assert tab.group.named is False
