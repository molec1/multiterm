"""The bash rc file that gives each tab its own command log."""

from multiterm.shell import SHELL_INIT, write_shell_init


def test_init_does_not_reload_history_before_bash_does():
    # Bash reads HISTFILE after --rcfile returns. history -r here would duplicate it.
    assert "history -c" in SHELL_INIT
    assert "history -r" not in SHELL_INIT
    assert 'export HISTFILE="$MULTITERM_HISTFILE"' in SHELL_INIT
    assert "history -a" in SHELL_INIT


def test_write_shell_init_lands_in_the_data_dir(monkeypatch, tmp_path):
    monkeypatch.setenv("MULTITERM_DATA", str(tmp_path))
    path = write_shell_init()
    assert path == tmp_path / "shell-init.sh"
    assert path.read_text(encoding="utf-8") == SHELL_INIT
