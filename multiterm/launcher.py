"""Desktop entry so Multiterm shows up in the application menu."""

from __future__ import annotations

import contextlib
import os
from pathlib import Path


def desktop_quote(value: str) -> str:
    """Quote one desktop-file Exec argument. The spec uses double quotes."""
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


ICON_NAME = "org.local.Multiterm"


def entry_point() -> Path:
    """Executable script at the project root, next to this package."""
    return Path(__file__).resolve().parent.parent / "bin" / "multiterm"


def icon_source() -> Path:
    """Scalable app icon shipped with the project."""
    return (
        Path(__file__).resolve().parent.parent
        / "data"
        / "icons"
        / "hicolor"
        / "scalable"
        / "apps"
        / f"{ICON_NAME}.svg"
    )


def install_icon() -> Path:
    """Copy the icon where the desktop and the icon theme can find it."""
    destination_dir = Path.home() / ".local" / "share" / "icons" / "hicolor" / "scalable" / "apps"
    destination_dir.mkdir(parents=True, exist_ok=True)
    destination = destination_dir / f"{ICON_NAME}.svg"
    destination.write_bytes(icon_source().read_bytes())
    with contextlib.suppress(OSError):
        os.chmod(destination, 0o644)
    return destination


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
Icon=org.local.Multiterm
StartupWMClass=org.local.Multiterm
Terminal=false
Categories=System;TerminalEmulator;GTK;
StartupNotify=true
Keywords=terminal;shell;tabs;history;
"""


def install_launcher() -> Path:
    """Write ``~/.local/share/applications/multiterm.desktop`` and return that path."""
    applications = Path.home() / ".local" / "share" / "applications"
    applications.mkdir(parents=True, exist_ok=True)
    install_icon()
    target = applications / "multiterm.desktop"
    target.write_text(desktop_entry_text(), encoding="utf-8")
    with contextlib.suppress(OSError):
        os.chmod(target, 0o644)
    return target


def _python() -> str:
    import sys

    return sys.executable or "/usr/bin/python3"
