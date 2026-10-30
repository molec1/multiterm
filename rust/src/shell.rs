//! Per-tab bash setup.
//!
//! Bash loads `HISTFILE` after the rc file returns, so the rc file clears the
//! in-memory history and does not read the file itself. Reading it here would
//! show every command twice.

use std::path::Path;

use crate::paths::data_dir;

const SHELL_INIT: &str = r#"# Multiterm tab init. Invoked as: bash --rcfile <this>
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
"#;

pub fn shell_init_path() -> std::path::PathBuf {
    data_dir().join("shell-init.sh")
}

pub fn write_shell_init() -> std::path::PathBuf {
    let path = shell_init_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, SHELL_INIT);
    let _ = std::fs::set_permissions(&path, {
        use std::os::unix::fs::PermissionsExt;
        std::fs::Permissions::from_mode(0o644)
    });
    path
}

pub fn prepare_history(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::OpenOptions::new().create(true).write(true).truncate(false).open(path);
    let _ = std::fs::set_permissions(path, {
        use std::os::unix::fs::PermissionsExt;
        std::fs::Permissions::from_mode(0o600)
    });
}

pub fn tab_environment(history: &Path) -> Vec<String> {
    let mut env: Vec<String> = std::env::vars().map(|(key, value)| format!("{key}={value}")).collect();
    let set = |env: &mut Vec<String>, key: &str, value: &str| {
        env.retain(|item| !item.starts_with(&format!("{key}=")));
        env.push(format!("{key}={value}"));
    };
    let history = history.display().to_string();
    set(&mut env, "TERM", "xterm-256color");
    set(&mut env, "MULTITERM_HISTFILE", &history);
    set(&mut env, "HISTFILE", &history);
    set(&mut env, "MULTITERM_READY_FILE", &format!("{history}.ready"));
    env
}
