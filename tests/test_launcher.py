"""Desktop entry quoting."""

from multiterm.launcher import desktop_entry_text, desktop_quote


def test_desktop_quote_escapes_spaces_and_quotes():
    assert desktop_quote("/home/alex/Documents/multi terminal/bin/multiterm") == (
        '"/home/alex/Documents/multi terminal/bin/multiterm"'
    )
    assert '\\"' in desktop_quote('say "hi"')


def test_entry_uses_the_project_launcher():
    text = desktop_entry_text()
    assert "Name=Multiterm" in text
    assert "bin/multiterm" in text
    assert "Terminal=false" in text
    assert "Icon=org.local.Multiterm" in text
