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
    inputs.nixvim.homeModules.nixvim
  ];

  # Disable zellij auto-start on zsh for kt-prox-nix
  programs.zellij.enableZshIntegration = lib.mkForce false;
}
