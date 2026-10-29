"""Filesystem locations and path display."""

from __future__ import annotations

import contextlib
import os
from pathlib import Path

APP_ID = "org.local.Multiterm"


def data_dir() -> Path:
    """Directory that holds the session file and per-tab command logs.

    ``MULTITERM_DATA`` overrides the location. Tests use that so they do not
    touch the real session in ``~/.local/share/multiterm``.
    """
    override = os.environ.get("MULTITERM_DATA")
    if override:
        return Path(override)
    base = os.environ.get("XDG_DATA_HOME", str(Path.home() / ".local" / "share"))
    return Path(base) / "multiterm"


def ensure_data() -> Path:
    """Create the data directory and the history folder, private to the user."""
    root = data_dir()
    history = root / "history"
    history.mkdir(parents=True, exist_ok=True)
    _chmod(root, 0o700)
    _chmod(history, 0o700)
    return root


def display_path(path: str) -> str:
    """Show ``path`` relative to the home directory, as a shell prompt would."""
    home = str(Path.home())
    if path == home:
        return "~"
    prefix = home + os.sep
    if path.startswith(prefix):
        return "~" + os.sep + path[len(prefix) :]
    return path


def folder_uri(path: str) -> str:
    """File URI for the default file manager."""
    return Path(path).resolve().as_uri()


def existing_directory(path: str) -> str:
    """Return ``path`` when it is a directory, otherwise the home directory."""
    if path and os.path.isdir(path):
        return path
    return str(Path.home())


def pid_cwd(pid: int | None) -> str | None:
    """Working directory of a live process, or ``None`` if it has exited."""
    if not pid:
        return None
    try:
        target = os.readlink(f"/proc/{pid}/cwd")
    except OSError:
        return None
    if os.path.isdir(target):
        return target
    return None


def _chmod(path: Path, mode: int) -> None:
    with contextlib.suppress(OSError):
        os.chmod(path, mode)
