"""Saved window: which tabs are open, how they are grouped, and their folders."""

from __future__ import annotations

import contextlib
import json
import os
from dataclasses import dataclass, field

from multiterm.paths import data_dir, ensure_data

SESSION_VERSION = 2
# The first version of groups always stored one group named Tabs. That was not
# a group the user created, so a lone "Tabs" group is restored as loose tabs.
_IMPLICIT_GROUP_NAMES = {"", "Tabs"}


@dataclass
class TabRecord:
    """One restored tab. The id is also the history-file name."""

    id: str
    cwd: str
    title: str = ""


@dataclass
class GroupRecord:
    """Tabs that belong together. ``named`` is false for the loose tabs.

    Loose tabs are the normal notebook. A named group is one the user created.
    ``active`` is the selected tab inside the group.
    """

    id: str
    name: str
    tabs: list[TabRecord] = field(default_factory=list)
    active: int = 0
    named: bool = False


@dataclass
class Session:
    groups: list[GroupRecord] = field(default_factory=list)
    active_group: int = 0
    width: int = 1000
    height: int = 640
    version: int = SESSION_VERSION

    @property
    def tabs(self) -> list[TabRecord]:
        return [tab for group in self.groups for tab in group.tabs]

    def window_size(self) -> tuple[int, int] | None:
        if self.width > 200 and self.height > 150:
            return self.width, self.height
        return None


def session_path():
    return data_dir() / "session.json"


def load_session() -> Session | None:
    """Read the session file. ``None`` means there is nothing usable to restore.

    Version 1 stored a flat tab list. It is restored as loose tabs, with no group.
    """
    try:
        raw = json.loads(session_path().read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(raw, dict):
        return None
    if isinstance(raw.get("groups"), list):
        groups = _parse_groups(raw["groups"])
    elif isinstance(raw.get("tabs"), list):
        groups = [_flat_group(raw)]
    else:
        return None
    return Session(
        groups=groups,
        active_group=_int(raw.get("active_group"), 0),
        width=_int(raw.get("width"), 1000),
        height=_int(raw.get("height"), 640),
        version=SESSION_VERSION,
    )


def save_session(session: Session) -> None:
    """Write the session atomically so a crash mid-save cannot truncate it."""
    ensure_data()
    path = session_path()
    temporary = path.with_suffix(".json.tmp")
    payload = {
        "version": SESSION_VERSION,
        "active_group": session.active_group,
        "width": session.width,
        "height": session.height,
        "groups": [
            {
                "id": group.id,
                "name": group.name,
                "named": group.named,
                "active": group.active,
                "tabs": [{"id": tab.id, "cwd": tab.cwd, "title": tab.title} for tab in group.tabs],
            }
            for group in session.groups
        ],
    }
    temporary.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    with contextlib.suppress(OSError):
        os.chmod(temporary, 0o600)
    os.replace(temporary, path)


def gc_history(keep_ids: set[str]) -> None:
    """Delete history files for tabs that are no longer part of the session."""
    history = data_dir() / "history"
    if not history.is_dir():
        return
    for entry in history.iterdir():
        if entry.name.endswith(".ready"):
            stem = entry.name[: -len(".ready")]
            if stem not in keep_ids:
                entry.unlink(missing_ok=True)
            continue
        if entry.is_file() and entry.name not in keep_ids:
            entry.unlink(missing_ok=True)


def _parse_groups(items: list) -> list[GroupRecord]:
    pending: list[tuple[GroupRecord, object]] = []
    for item in items:
        if not isinstance(item, dict):
            continue
        group = _parse_group(item)
        if group is not None:
            pending.append((group, item.get("named")))
    only = len(pending) == 1
    groups: list[GroupRecord] = []
    for group, explicit in pending:
        if isinstance(explicit, bool):
            group.named = explicit
        elif only and group.name in _IMPLICIT_GROUP_NAMES:
            group.named = False
            group.name = ""
        else:
            group.named = True
        groups.append(group)
    return groups


def _parse_group(item: dict) -> GroupRecord | None:
    group_id = item.get("id")
    if not isinstance(group_id, str) or not _safe_id(group_id):
        return None
    name = item.get("name")
    tabs = item.get("tabs")
    return GroupRecord(
        id=group_id,
        name=name if isinstance(name, str) else "",
        tabs=_parse_tabs(tabs if isinstance(tabs, list) else []),
        active=_int(item.get("active"), 0),
    )


def _flat_group(raw: dict) -> GroupRecord:
    return GroupRecord(
        id="tabs",
        name="",
        named=False,
        tabs=_parse_tabs(raw["tabs"]),
        active=_int(raw.get("active"), 0),
    )


def _parse_tabs(items: list) -> list[TabRecord]:
    tabs: list[TabRecord] = []
    for item in items:
        if not isinstance(item, dict):
            continue
        tab_id = item.get("id")
        if not isinstance(tab_id, str) or not _safe_id(tab_id):
            continue
        cwd = item.get("cwd")
        title = item.get("title")
        tabs.append(
            TabRecord(
                id=tab_id,
                cwd=cwd if isinstance(cwd, str) else "",
                title=title.strip() if isinstance(title, str) else "",
            )
        )
    return tabs


def _int(value: object, fallback: int) -> int:
    return value if isinstance(value, int) else fallback


def _safe_id(value: str) -> bool:
    return bool(value) and value not in {".", ".."} and "/" not in value and "\\" not in value
