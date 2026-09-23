{ config, ... }:

{
  imports = [
    ../common
    ./cli
    ./gui
  ];

  # Resolved from 1Password on demand: `with-secrets <cmd>` / `secrets-load`.
  dotfiles.secrets.env = {
    BRAVE_API_KEY = "op://Personal/BraveAPI/credential";
    TELEGRAM_BOT_TOKEN = "op://Personal/Telegram/credential";
  };

  home.sessionVariables = {
    USE_SYMENGINE = "1";
    OLLAMA_HOST = "0.0.0.0";
  };

  programs.neovim.obsidianVaults = [
    {
      name = "Private";
      path = "${config.home.homeDirectory}/Library/Mobile Documents/iCloud~md~obsidian/Documents/Private";
    }
    {
      name = "Work";
      path = "${config.home.homeDirectory}/Library/Mobile Documents/iCloud~md~obsidian/Documents/Work";
    }
  ];

  home.sessionPath = [
    "${config.home.homeDirectory}/.local/bin"
    "${config.home.homeDirectory}/.deno/bin"
  ];
}
