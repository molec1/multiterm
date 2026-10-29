"""Path display and the private data directory."""

from pathlib import Path

from multiterm.paths import data_dir, display_path, existing_directory, folder_uri


def test_display_path_uses_tilde_for_home(monkeypatch, tmp_path):
    home = tmp_path / "home"
    home.mkdir()
    monkeypatch.setattr(Path, "home", staticmethod(lambda: home))
    assert display_path(str(home)) == "~"
    assert display_path(str(home / "src" / "app")) == "~/src/app"
    assert display_path("/etc") == "/etc"


def test_folder_uri_is_absolute():
    uri = folder_uri("/tmp")
    assert uri.startswith("file://")
    assert uri.endswith("/tmp") or uri.endswith("/private/tmp")


def test_missing_directory_falls_back_to_home(tmp_path, monkeypatch):
    home = tmp_path / "home"
    home.mkdir()
    monkeypatch.setattr("multiterm.paths.Path.home", lambda: home)
    assert existing_directory(str(tmp_path / "gone")) == str(home)
    assert existing_directory(str(tmp_path)) == str(tmp_path)


def test_data_dir_honors_override(monkeypatch, tmp_path):
    monkeypatch.setenv("MULTITERM_DATA", str(tmp_path / "state"))
    assert data_dir() == tmp_path / "state"
