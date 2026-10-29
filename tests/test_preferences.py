"""The preferences window reads the GNOME Terminal profile and does not rewrite it on open."""

from __future__ import annotations

import os

import pytest

gi = pytest.importorskip("gi")
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk
from multiterm.preferences import PROFILE_KEYS, SETTINGS_KEYS, PreferencesDialog
from multiterm.style import gnome_profile_settings, terminal_settings

pytestmark = pytest.mark.skipif(not os.environ.get("DISPLAY"), reason="needs a display")


def test_profile_keys_exist_and_opening_the_dialog_does_not_write():
    profile = gnome_profile_settings()
    terminal = terminal_settings()
    if profile is None or terminal is None:
        pytest.skip("GNOME Terminal settings are not available")
    before = {key: profile.get_value(key) for key in PROFILE_KEYS}
    before_settings = {key: terminal.get_value(key) for key in SETTINGS_KEYS}
    parent = Gtk.Window()
    try:
        dialog = PreferencesDialog(parent)
        dialog.show_all()
        dialog.destroy()
    finally:
        parent.destroy()
        for key, value in before.items():
            if profile.get_value(key).unpack() != value.unpack():
                profile.set_value(key, value)
        for key, value in before_settings.items():
            if terminal.get_value(key).unpack() != value.unpack():
                terminal.set_value(key, value)
    for key, value in before.items():
        assert profile.get_value(key).unpack() == value.unpack(), key
    for key, value in before_settings.items():
        assert terminal.get_value(key).unpack() == value.unpack(), key
