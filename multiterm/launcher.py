"""Desktop entry so Multiterm shows up in the application menu."""

from __future__ import annotations

import contextlib
import os
from pathlib import Path


def desktop_quote(value: str) -> str:
    """Quote one desktop-file Exec argument. The spec uses double quotes."""
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def entry_point() -> Path:
    """Executable script at the project root, next to this package."""
    return Path(__file__).resolve().parent.parent / "bin" / "multiterm"


def desktop_entry_text(script: Path | None = None) -> str:
    target = script or entry_point()
    python = _python()
    return f"""[Desktop Entry]
Version=1.0
Type=Application
Name=Multiterm
GenericName=Terminal
Comment=Terminal tabs that remember their folder and command history
Exec={desktop_quote(python)} {desktop_quote(str(target))}
Icon=utilities-terminal
Terminal=false
Categories=System;TerminalEmulator;GTK;
StartupNotify=true
Keywords=terminal;shell;tabs;history;
"""


def install_launcher() -> Path:
    """Write ``~/.local/share/applications/multiterm.desktop`` and return that path."""
    applications = Path.home() / ".local" / "share" / "applications"
    applications.mkdir(parents=True, exist_ok=True)
    target = applications / "multiterm.desktop"
    target.write_text(desktop_entry_text(), encoding="utf-8")
    with contextlib.suppress(OSError):
        os.chmod(target, 0o644)
    return target


def _python() -> str:
    import sys

    return sys.executable or "/usr/bin/python3"
