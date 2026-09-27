# Sourced by zsh. No key listing for local 1Password.
[ -n "${_DOTFILES_SSH_AGENT_INITIALIZED:-}" ] && return 0
_DOTFILES_SSH_AGENT_INITIALIZED=1
export DEV_SSH_AGENT_SOCK="$HOME/.ssh/agent/current"
_dotfiles_local_agent="$HOME/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"
mkdir -p "$HOME/.ssh/agent"

if [ -n "${SSH_CONNECTION:-}" ]; then
  # An SSH session without forwarding must never acquire this host's agent.
  if [ -S "${SSH_AUTH_SOCK:-}" ] && [ "$SSH_AUTH_SOCK" != "$DEV_SSH_AGENT_SOCK" ]; then
    ln -sf "$SSH_AUTH_SOCK" "$DEV_SSH_AGENT_SOCK"
    export SSH_AUTH_SOCK="$DEV_SSH_AGENT_SOCK"
  fi
else
  if ! [ "$DEV_SSH_AGENT_SOCK" -ef "$_dotfiles_local_agent" ]; then
    # Mux panes may have no SSH_CONNECTION but still need the pinned forwarded
    # agent. Only exit 2 proves it unreachable; empty keys (1) and a timeout
    # (124/137) must not redirect approvals to this machine's 1Password.
    _dotfiles_agent_status=0
    SSH_AUTH_SOCK="$DEV_SSH_AGENT_SOCK" \
      @timeout@ --kill-after=0.5s 0.5s @sshAdd@ -l >/dev/null 2>&1 \
      || _dotfiles_agent_status=$?
    if [ "$_dotfiles_agent_status" -eq 2 ]; then
      ln -sf "$_dotfiles_local_agent" "$DEV_SSH_AGENT_SOCK"
    fi
    unset _dotfiles_agent_status
  fi
  export SSH_AUTH_SOCK="$DEV_SSH_AGENT_SOCK"
fi
unset _dotfiles_local_agent
