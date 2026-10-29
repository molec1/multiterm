"""Session file round-trip and history cleanup."""

import json

from multiterm.paths import ensure_data
from multiterm.session import (
    GroupRecord,
    Session,
    TabRecord,
    gc_history,
    load_session,
    save_session,
)


def test_round_trip(monkeypatch, tmp_path):
    monkeypatch.setenv("MULTITERM_DATA", str(tmp_path))
    session = Session(
        groups=[
            GroupRecord(
                id="main",
                name="Work",
                named=True,
                active=1,
                tabs=[
                    TabRecord(id="one", cwd="/tmp/one", title="api"),
                    TabRecord(id="two", cwd="/tmp/two"),
                ],
            ),
            GroupRecord(
                id="side",
                name="Ops",
                named=True,
                tabs=[TabRecord(id="three", cwd="/tmp/three")],
            ),
        ],
        active_group=1,
        width=800,
        height=600,
    )
    save_session(session)
    loaded = load_session()
    assert loaded is not None
    assert loaded.groups == session.groups
    assert loaded.active_group == 1
    assert loaded.tabs == session.tabs
    assert loaded.window_size() == (800, 600)


def test_version_1_session_becomes_one_group(monkeypatch, tmp_path):
    monkeypatch.setenv("MULTITERM_DATA", str(tmp_path))
    ensure_data()
    payload = {
        "version": 1,
        "active": 0,
        "width": 900,
        "height": 500,
        "tabs": [{"id": "kept", "cwd": "/tmp/kept", "title": "  api  "}],
    }
    (tmp_path / "session.json").write_text(json.dumps(payload), encoding="utf-8")
    loaded = load_session()
    assert loaded is not None
    assert len(loaded.groups) == 1
    group = loaded.groups[0]
    assert group.name == ""
    assert group.named is False
    assert group.tabs == [TabRecord(id="kept", cwd="/tmp/kept", title="api")]


def test_lone_tabs_group_is_loose(monkeypatch, tmp_path):
    monkeypatch.setenv("MULTITERM_DATA", str(tmp_path))
    ensure_data()
    payload = {
        "version": 2,
        "active_group": 0,
        "width": 900,
        "height": 500,
        "groups": [{"id": "tabs", "name": "Tabs", "tabs": [{"id": "kept", "cwd": "/tmp"}]}],
    }
    (tmp_path / "session.json").write_text(json.dumps(payload), encoding="utf-8")
    loaded = load_session()
    assert loaded is not None
    assert loaded.groups[0].named is False
    assert loaded.groups[0].name == ""


def test_explicit_name_without_a_flag_is_a_group(monkeypatch, tmp_path):
    monkeypatch.setenv("MULTITERM_DATA", str(tmp_path))
    ensure_data()
    payload = {
        "version": 2,
        "active_group": 0,
        "width": 900,
        "height": 500,
        "groups": [
            {"id": "work", "name": "Work", "tabs": [{"id": "one", "cwd": "/tmp"}]},
            {"id": "ops", "name": "Ops", "tabs": [{"id": "two", "cwd": "/tmp"}]},
        ],
    }
    (tmp_path / "session.json").write_text(json.dumps(payload), encoding="utf-8")
    loaded = load_session()
    assert loaded is not None
    assert [group.named for group in loaded.groups] == [True, True]


def test_explicit_named_flag_wins(monkeypatch, tmp_path):
    monkeypatch.setenv("MULTITERM_DATA", str(tmp_path))
    ensure_data()
    payload = {
        "version": 2,
        "active_group": 0,
        "width": 900,
        "height": 500,
        "groups": [
            {
                "id": "tabs",
                "name": "Tabs",
                "named": True,
                "tabs": [{"id": "kept", "cwd": "/tmp"}],
            }
        ],
    }
    (tmp_path / "session.json").write_text(json.dumps(payload), encoding="utf-8")
    loaded = load_session()
    assert loaded is not None
    assert loaded.groups[0].named is True
    assert loaded.groups[0].name == "Tabs"


def test_corrupt_session_is_ignored(monkeypatch, tmp_path):
    monkeypatch.setenv("MULTITERM_DATA", str(tmp_path))
    ensure_data()
    (tmp_path / "session.json").write_text("{", encoding="utf-8")
    assert load_session() is None


def test_rejects_tab_ids_that_escape_the_history_directory(monkeypatch, tmp_path):
    monkeypatch.setenv("MULTITERM_DATA", str(tmp_path))
    ensure_data()
    payload = {
        "version": 1,
        "active": 0,
        "width": 900,
        "height": 500,
        "tabs": [{"id": "../x", "cwd": "/tmp"}],
    }
    (tmp_path / "session.json").write_text(json.dumps(payload), encoding="utf-8")
    loaded = load_session()
    assert loaded is not None
    assert loaded.tabs == []


def test_gc_drops_logs_for_closed_tabs(monkeypatch, tmp_path):
    monkeypatch.setenv("MULTITERM_DATA", str(tmp_path))
    history = tmp_path / "history"
    history.mkdir()
    (history / "keep").write_text("ls\n", encoding="utf-8")
    (history / "drop").write_text("pwd\n", encoding="utf-8")
    (history / "drop.ready").write_text("", encoding="utf-8")
    gc_history({"keep"})
    assert (history / "keep").exists()
    assert not (history / "drop").exists()
    assert not (history / "drop.ready").exists()
