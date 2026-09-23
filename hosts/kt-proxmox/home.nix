{ lib, ... }:

{
  home.sessionVariables.EDITOR = "nvim";

  xdg.enable = true;

  # Disable zellij auto-start on zsh for kt-proxmox
  programs.zellij.enableZshIntegration = lib.mkForce false;
}
