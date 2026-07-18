{ lib, ... }:

{
  home.sessionVariables = {
    EDITOR = "nvim";
    NIXPKGS_ALLOW_UNFREE = 1;
  };

  programs.home-manager.enable = true;

  xdg = {
    enable = true;
    userDirs.extraConfig = {
      desktop = "$HOME/Desktop";
      download = "$HOME/Downloads";
      documents = "$HOME/Documents";
      music = "$HOME/Music";
      videos = "$HOME/Videos";
    };
  };

  # Disable zellij auto-start on zsh for kt-proxmox
  programs.zellij.enableZshIntegration = lib.mkForce false;
}
