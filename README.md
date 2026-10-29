# Multiterm

Terminal tabs for Ubuntu that reopen where you left them.

Each tab remembers its folder and keeps a separate command log. Opening the app again restores those tabs. The up arrow and `history` show only the commands from that tab. A folder icon in the header opens the current folder in the file manager.

Running programs are not restored. A command is stored once it has finished and the shell has printed the next prompt.

## Requirements

Ubuntu, with the same libraries as GNOME Terminal:

- Python 3.11+
- `python3-gi`
- `gir1.2-gtk-3.0`
- `gir1.2-vte-2.91`

Preferences follow the GNOME Terminal window: General, Shortcuts, and a profile's Text, Colors, Scrolling, Command, and Compatibility pages. They read and write the same settings, so a change shows up in GNOME Terminal as well. Colors, font, cursor, and scrollbar follow the default profile, including its light or dark variant.

## Run

```bash
./bin/multiterm
```

The first launch also installs a menu entry named Multiterm. To refresh that entry only:

```bash
./bin/multiterm --install-launcher
```

Double-click a tab, or right-click it and choose Rename, to give it a name. An empty name shows the folder again. The full path stays in the tooltip and in the header.

Tabs are not grouped until you ask. Ctrl+Shift+G, the header button, or New Group in the tab menu puts the current tab in a group named Group 1, Group 2, and so on. Groups sit in a tab strip above the terminal tabs. Drag a tab onto a group to move it. Right-click a group to rename it, ungroup it, or close every terminal in it.

A new tab uses the current folder unless the GNOME Terminal profile says not to preserve the working directory.

| Shortcut | Action |
| --- | --- |
| Ctrl+Shift+T | New tab |
| Ctrl+Shift+W | Close tab |
| Ctrl+Shift+O | Open the folder in Files |
| Ctrl+Shift+G | New tab group |
| Ctrl+Shift+C / V | Copy / paste |
| Alt+1 … Alt+9 | Switch tab |

Session data lives in `~/.local/share/multiterm`.

## Development

```bash
python3 -m venv --system-site-packages .venv
.venv/bin/pip install pytest ruff
.venv/bin/ruff check .
.venv/bin/ruff format --check .
.venv/bin/pytest
```

The virtualenv is created with `--system-site-packages` so the tests can import the system GTK and VTE bindings. `tests/test_restore.py` opens a real window, so it needs a display. The other tests do not.
