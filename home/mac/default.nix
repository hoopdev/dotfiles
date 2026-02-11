{ lib, pkgs, ... }:

{
  imports = [
    ../common
    ./gui
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
    initContent = ''
      export LANG=ja_JP.utf8
      eval "$(/opt/homebrew/bin/brew shellenv)"

      # 1Password secrets cache (~/.op-secrets)
      # Delete this file to force re-fetch: rm ~/.op-secrets
      if [[ ! -f ~/.op-secrets ]]; then
        op read "op://Personal/Anthropic/credential" > /dev/null 2>&1 && {
          cat > ~/.op-secrets <<EOF
      export ANTHROPIC_API_KEY=$(op read "op://Personal/Anthropic/credential")
      export BRAVE_API_KEY=$(op read "op://Personal/BraveAPI/credential")
      export TELEGRAM_BOT_TOKEN=$(op read "op://Personal/Telegram/credential")
      export GATEWAY_AUTH_TOKEN=$(op read "op://Personal/OpenclawGateway/credential")
      EOF
          chmod 600 ~/.op-secrets
        }
      fi
      [[ -f ~/.op-secrets ]] && source ~/.op-secrets
    '';
  };

}
