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
    '';
  };

  # Karabiner Elements設定
  home.file.".config/karabiner/karabiner.json" = {
    source = ./karabiner.json;
    onChange = ''
      /bin/launchctl kickstart -k gui/$(id -u)/org.pqrs.karabiner.karabiner_console_user_server 2>/dev/null || true
    '';
  };
}
