"""Application object and the process entry point."""

from __future__ import annotations

import os
import sys
from collections.abc import Callable

import gi

gi.require_version("Gtk", "3.0")

from gi.repository import Gio, GLib, Gtk

from multiterm.launcher import install_launcher
from multiterm.paths import APP_ID, ensure_data
from multiterm.shell import write_shell_init
from multiterm.style import apply_terminal_theme_preference
from multiterm.window import MainWindow


class MultitermApp(Gtk.Application):
    """One process, one window. A second launch raises the existing window."""

    def __init__(
        self,
        *,
        testing: bool = False,
        bench: bool = False,
        on_ready: Callable[[MainWindow], None] | None = None,
    ) -> None:
        separate = testing or bench
        flags = Gio.ApplicationFlags.NON_UNIQUE if separate else Gio.ApplicationFlags.FLAGS_NONE
        if testing:
            app_id = f"{APP_ID}.Test"
        elif bench:
            app_id = f"{APP_ID}.Bench"
        else:
            app_id = APP_ID
        super().__init__(application_id=app_id, flags=flags)
        self.window: MainWindow | None = None
        self.exit_code = 1 if testing else 0
        self._on_ready = on_ready
        self._bench = bench

    def do_activate(self) -> None:
        if self.window is None:
            ensure_data()
            write_shell_init()
            apply_terminal_theme_preference()
            self.window = MainWindow(self)
            self.window.show_all()
            tab = self.window.current_tab()
            if tab is not None:
                tab.terminal.grab_focus()
            if self._on_ready is not None:
                self._on_ready(self.window)
        self.window.present()
        if self._bench and not getattr(self, "_bench_started", False):
            self._bench_started = True
            print("BENCH ready", flush=True)
            GLib.idle_add(self.window.continue_bench)


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv if argv is None else argv)
    GLib.set_prgname("multiterm")
    GLib.set_application_name("Multiterm")
    bench = os.environ.get("MULTITERM_BENCH") == "1"
    if "--install-launcher" in argv:
        return _install()
    if not bench:
        try:
            _install()
        except OSError as error:
            print(f"Could not install the launcher: {error}", file=sys.stderr)
    app = MultitermApp(bench=bench)
    app.run([argv[0]])
    return app.exit_code


def _install() -> int:
    try:
        install_launcher()
    except OSError as error:
        print(f"Could not install the launcher: {error}", file=sys.stderr)
        return 1
    return 0
