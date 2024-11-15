{
  pkgs,
  lib,
  username,
  inputs,
  ...
}:
{
  home = rec {
    inherit username;
    homeDirectory = "/Users/${username}";
    stateVersion = "24.05";
    sessionVariables = {
      EDITOR = "nvim";
      NIXPKGS_ALLOW_UNFREE = 1;
      OneDrive = "/Users/ktaga/Library/CloudStorage/OneDrive-KyotoUniversity";
      USE_SYMENGINE = 1;
      OLLAMA_HOST = "0.0.0.0";
    };
    sessionPath = [
      "/Users/ktaga/.local/bin"
      "/Users/ktaga/.deno/bin"
    ];
  };

  # home-managerの有効化
  programs.home-manager.enable = true;

  imports = [
    ../../home/common/cli
    ../../home/common/gui
    ../../home/mac/gui
    inputs.nix-colors.homeManagerModules.default
    #inputs.nixvim.homeManagerModules.nixvim
  ];
  programs.zsh.initExtra =
    # bash
    ''
      export LANG=ja_JP.utf8
      eval "$(/opt/homebrew/bin/brew shellenv)"
    '';
  colorScheme = inputs.nix-colors.colorSchemes.nord;

  home.packages = with pkgs; [
    inputs.nixvim.packages.aarch64-darwin.default
  ];
}
