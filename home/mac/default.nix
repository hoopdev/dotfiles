{ config, ... }:

{
  imports = [
    ../common
    ./cli
    ./gui
  ];

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
