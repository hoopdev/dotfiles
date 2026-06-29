{ ... }:

{
  imports = [
    ../common
    ./gui
    ./coder.nix
  ];

  home.sessionVariables = {
    # OneDrive = "/Users/ktaga/Library/CloudStorage/OneDrive-KyotoUniversity";
    USE_SYMENGINE = "1";
    OLLAMA_HOST = "0.0.0.0";
  };

  home.sessionPath = [
    "/Users/ktaga/.local/bin"
    "/Users/ktaga/.deno/bin"
  ];

  programs.zsh = {
    enable = true;
    # SSH agent strategy (see also home/common/cli/ssh.nix):
    #   Neither ~/.ssh/config nor config.local sets IdentityAgent anywhere.
    #   Instead, $SSH_AUTH_SOCK is the single source of truth:
    #   - Local login  → point SSH_AUTH_SOCK at the 1Password agent socket so all
    #                    SSH connections (github.com, remote hosts, …) use local 1Password.
    #   - SSH session  → sshd injects the ForwardAgent socket into SSH_AUTH_SOCK
    #                    automatically; do NOT override it, so the caller's keys are used.
    loginExtra = ''
      if [[ -z "$SSH_CLIENT" && -z "$SSH_TTY" ]]; then
        export SSH_AUTH_SOCK="$HOME/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"
      fi
    '';
    initContent = ''
      export LANG=ja_JP.utf8
      eval "$(/opt/homebrew/bin/brew shellenv)"

      # 1Password secrets cache (~/.op-secrets)
      # Delete this file to force re-fetch: rm ~/.op-secrets
      if [[ ! -f ~/.op-secrets ]]; then
        op read "op://Personal/Anthropic/credential" > /dev/null 2>&1 && {
          cat > ~/.op-secrets <<EOF
      export BRAVE_API_KEY=$(op read "op://Personal/BraveAPI/credential")
      export TELEGRAM_BOT_TOKEN=$(op read "op://Personal/Telegram/credential")
      export GATEWAY_AUTH_TOKEN=$(op read "op://Personal/OpenclawGateway/credential")
      export OPENROUTER_API_KEY=$(op read "op://Personal/Openrouter/credential")
      EOF
          chmod 600 ~/.op-secrets
        }
      fi
      [[ -f ~/.op-secrets ]] && source ~/.op-secrets
    '';
  };

}
