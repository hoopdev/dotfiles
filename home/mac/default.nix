{ config, pkgs, ... }:
{
  imports = [
    ../common
    ./gui
  ];

  home = {
    sessionVariables = {
      OneDrive = "/Users/ktaga/Library/CloudStorage/OneDrive-KyotoUniversity";
      USE_SYMENGINE = 1;
      OLLAMA_HOST = "0.0.0.0";
    };

    sessionPath = [
      "/Users/ktaga/.local/bin"
      "/Users/ktaga/.deno/bin"
    ];
  };

  programs.zsh.initExtra = ''
    export LANG=ja_JP.utf8
    eval "$(/opt/homebrew/bin/brew shellenv)"
  '';
}
