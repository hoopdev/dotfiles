{
  username,
  inputs,
  lib,
  ...
}:
{
  home = rec {
    inherit username;
    homeDirectory = "/home/${username}";
    stateVersion = "24.05";
    sessionVariables = {
      EDITOR = "nvim";
      NIXPKGS_ALLOW_UNFREE = 1;
    };
    sessionPath =
      [
      ];
  };

  xdg = {
    enable = true;
    userDirs = {
      extraConfig = {
        desktop = "/home/ktaga/Desktop";
        download = "/home/ktaga/Downloads";
        documents = "/home/ktaga/Documents";
        music = "/home/ktaga/Music";
        videos = "/home/ktaga/Videos";
      };
    };
  };

  programs.home-manager.enable = true;

  imports = [
    ../../home/common/cli
    ../../home/nixos/cli
    #../../home/common/gui
    #../../home/nixos/gui
    inputs.nix-colors.homeManagerModules.default
    inputs.nixvim.homeModules.nixvim
  ];

  colorScheme = inputs.nix-colors.colorSchemes.nord;

  # Disable zellij auto-start on zsh for kt-prox-nix
  programs.zellij.enableZshIntegration = lib.mkForce false;

  home.packages = [
    #inputs.hyprpanel.packages.x86_64-linux.default
  ];
}
