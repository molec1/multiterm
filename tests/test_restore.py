"""Restore a tab's folder and its own command history. Needs a display."""

from __future__ import annotations

import json
import os
import shlex
from pathlib import Path

import pytest

gi = pytest.importorskip("gi")
gi.require_version("Gtk", "3.0")
from gi.repository import GLib
from multiterm.app import MultitermApp
from multiterm.paths import data_dir, folder_uri
from multiterm.style import terminal_prefers_dark

pytestmark = pytest.mark.skipif(not os.environ.get("DISPLAY"), reason="needs a display")


class _Probe:
    """Drive one restored tab: history must load once, then cd must stick."""

    def __init__(self, window, root: Path) -> None:
        self.window = window
        self.root = root
        self.repo = root / "repo"
        self.repo.mkdir(parents=True, exist_ok=True)
        self.phase = "wait-ready"
        self.deadline = GLib.get_monotonic_time() + 40 * 1000 * 1000
        window.set_keep_below(True)
        window.set_skip_taskbar_hint(True)
        window.move(0, 4000)
        window.app.exit_code = 1
        self.error: str | None = None

    def tick(self) -> bool:
        try:
            return self._step()
        except Exception as error:
            return self._fail(f"{type(error).__name__}: {error}")

    def _step(self) -> bool:
        if GLib.get_monotonic_time() > self.deadline:
            return self._fail(f"timeout in {self.phase}")
        tab = self.window.current_tab()
        if tab is None:
            return self._fail("no tab")
        if self.phase == "wait-ready":
            if tab.pid and tab.ready_path.exists():
                command = (
                    "history > "
                    + shlex.quote(str(self.root / "history-out.txt"))
                    + '; printf %s "$TERM" > '
                    + shlex.quote(str(self.root / "term.txt"))
                    + "; cd "
                    + shlex.quote(str(self.repo))
                    + "; echo multiterm-marker\n"
                )
                tab.terminal.feed_child(command.encode())
                self.phase = "wait-result"
            return True
        if tab.history_path.exists():
            history = tab.history_path.read_text(encoding="utf-8", errors="replace")
        else:
            history = ""
        history_out = self.root / "history-out.txt"
        term_file = self.root / "term.txt"
        if "multiterm-marker" not in history or not history_out.exists() or not term_file.exists():
            return True
        if tab.cwd != str(self.repo):
            return True
        shown = history_out.read_text(encoding="utf-8", errors="replace")
        if shown.count("seeded-from-disk") != 1:
            return self._fail(f"seed count {shown.count('seeded-from-disk')}\n{shown}")
        term = term_file.read_text(encoding="utf-8", errors="replace").strip()
        if "256color" not in term:
            return self._fail(f"TERM={term!r}")
        self.window.save()
        stored = json.loads((data_dir() / "session.json").read_text(encoding="utf-8"))
        groups = stored.get("groups")
        if (
            not isinstance(groups, list)
            or len(groups) != 1
            or groups[0]["tabs"][0]["cwd"] != str(self.repo)
        ):
            return self._fail(f"session {stored}")
        uri = folder_uri(tab.cwd)
        if not uri.startswith("file://") or not uri.endswith("/repo"):
            return self._fail(f"uri {uri}")
        background = self.window._look.background
        luminance = background.red + background.green + background.blue
        if terminal_prefers_dark() and luminance > 1.5:
            return self._fail(f"terminal background is light ({background.to_string()})")
        if not terminal_prefers_dark() and luminance < 1.5:
            return self._fail(f"terminal background is dark ({background.to_string()})")
        return self._ok()

    def _fail(self, message: str) -> bool:
        self.error = message
        self.window.quitting = True
        self.window.app.exit_code = 1
        self.window.app.quit()
        return False

    def _ok(self) -> bool:
        self.window.quitting = True
        self.window.save()
        self.window.app.exit_code = 0
        self.window.app.quit()
        return False


def test_restores_folder_history_and_theme(tmp_path, monkeypatch):
    root = tmp_path / "run"
    data = root / "data"
    start = root / "start"
    (data / "history").mkdir(parents=True)
    start.mkdir()
    (data / "history" / "tabseed").write_text("echo seeded-from-disk\n", encoding="utf-8")
    session = {
        "version": 1,
        "active": 0,
        "width": 900,
        "height": 500,
        "tabs": [{"id": "tabseed", "cwd": str(start)}],
    }
    (data / "session.json").write_text(json.dumps(session), encoding="utf-8")
    monkeypatch.setenv("MULTITERM_DATA", str(data))

    probe_box: dict[str, _Probe] = {}

    def on_ready(window) -> None:
        probe = _Probe(window, root)
        probe_box["probe"] = probe
        GLib.timeout_add(200, probe.tick)

    app = MultitermApp(testing=True, on_ready=on_ready)
    app.run(["multiterm"])
    probe = probe_box["probe"]
    assert probe.error is None, probe.error
    assert app.exit_code == 0
