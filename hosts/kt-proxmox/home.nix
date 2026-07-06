{ lib, ... }:

{
  imports = [
    ../../home/nixos/headless.nix
    ../../home/nixos/cli # ollama (proxmox-only)
  ];

  home.sessionVariables = {
    EDITOR = "nvim";
    NIXPKGS_ALLOW_UNFREE = 1;
  };

  programs.home-manager.enable = true;

  xdg = {
    enable = true;
    userDirs.extraConfig = {
      desktop = "/home/ktaga/Desktop";
      download = "/home/ktaga/Downloads";
      documents = "/home/ktaga/Documents";
      music = "/home/ktaga/Music";
      videos = "/home/ktaga/Videos";
    };
  };

  # Disable zellij auto-start on zsh for kt-proxmox
  programs.zellij.enableZshIntegration = lib.mkForce false;
}
