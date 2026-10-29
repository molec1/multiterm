"""Per-tab bash setup.

Each tab gets its own ``HISTFILE``. Bash loads that file *after* the rc file
returns, so the rc file must ``history -c`` and must not ``history -r``.
Doing both loads every line twice.
"""

from __future__ import annotations

import contextlib
import os
from pathlib import Path

from multiterm.paths import data_dir

SHELL_INIT_NAME = "shell-init.sh"

# Sourced via `bash --rcfile`. That flag replaces both /etc/bash.bashrc and
# ~/.bashrc, so this script reads them itself and then pins HISTFILE.
SHELL_INIT = r"""# Multiterm tab init. Invoked as: bash --rcfile <this>
if [ -f /etc/bash.bashrc ]; then
    . /etc/bash.bashrc
fi
if [ -n "${HOME:-}" ] && [ -f "$HOME/.bashrc" ]; then
    . "$HOME/.bashrc"
fi

if [ -n "${MULTITERM_HISTFILE:-}" ]; then
    export HISTFILE="$MULTITERM_HISTFILE"
    : "${HISTSIZE:=10000}"
    : "${HISTFILESIZE:=20000}"
    export HISTSIZE HISTFILESIZE
    shopt -s histappend
    history -c
    __multiterm_prompt_hook() {
        history -a
        if [ -x /usr/libexec/vte-urlencode-cwd ]; then
            printf '\033]7;file://%s%s\033\\' "${HOSTNAME}" "$(/usr/libexec/vte-urlencode-cwd)"
        fi
    }
    if [[ "$(declare -p PROMPT_COMMAND 2>/dev/null)" == declare\ -a* ]]; then
        __mt_have=0
        for __mt_item in "${PROMPT_COMMAND[@]}"; do
            if [ "$__mt_item" = "__multiterm_prompt_hook" ]; then
                __mt_have=1
            fi
        done
        if [ "$__mt_have" -eq 0 ]; then
            PROMPT_COMMAND=(__multiterm_prompt_hook "${PROMPT_COMMAND[@]}")
        fi
        unset __mt_have __mt_item
    else
        case ";${PROMPT_COMMAND:-};" in
            *";__multiterm_prompt_hook;"*) ;;
            *) PROMPT_COMMAND="__multiterm_prompt_hook${PROMPT_COMMAND:+;$PROMPT_COMMAND}" ;;
        esac
    fi
fi

if [ -n "${MULTITERM_READY_FILE:-}" ]; then
    : > "$MULTITERM_READY_FILE"
fi
"""


def shell_init_path() -> Path:
    return data_dir() / SHELL_INIT_NAME


def write_shell_init() -> Path:
    """Install the rc file next to the session data and return its path."""
    path = shell_init_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(SHELL_INIT)
    with contextlib.suppress(OSError):
        os.chmod(path, 0o644)
    return path


def tab_environment(history_path: Path) -> list[str]:
    """Environment for one tab. ``HISTFILE`` is set before bash reads its rc file."""
    env = os.environ.copy()
    env["TERM"] = "xterm-256color"
    env["MULTITERM_HISTFILE"] = str(history_path)
    env["HISTFILE"] = str(history_path)
    env["MULTITERM_READY_FILE"] = str(history_path) + ".ready"
    return [f"{key}={value}" for key, value in env.items() if value is not None]
