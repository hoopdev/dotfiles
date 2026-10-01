# Sourced by zsh. Agent selection belongs to this session, never a shared link.
[ -n "${_DOTFILES_SSH_AGENT_INITIALIZED:-}" ] && return 0
_DOTFILES_SSH_AGENT_INITIALIZED=1

if [ -n "${SSH_CONNECTION:-}" ]; then
  # Keep sshd's socket unchanged. An empty value explicitly records that this
  # session has no forwarding, even if a mux pane later drops SSH_CONNECTION.
  export DEV_SSH_AGENT_SOCK="${SSH_AUTH_SOCK:-}"
else
  # Child mux panes inherit the selected socket (including an explicit empty
  # value). A disconnected forwarded agent must not become local 1Password.
  if [ "${DEV_SSH_AGENT_SOCK+x}" != x ]; then
    export DEV_SSH_AGENT_SOCK="$HOME/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"
  fi
  export SSH_AUTH_SOCK="$DEV_SSH_AGENT_SOCK"
fi
