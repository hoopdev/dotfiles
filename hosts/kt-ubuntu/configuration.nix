# Ubuntu NixOS configuration (without WSL, without desktop environment)
# Inherits most settings from kt-wsl but removes WSL-specific modules

{
  inputs,
  config,
  lib,
  pkgs,
  ...
}:

{
  imports = [
    ../../lib/nixos-common.nix
  ];

  # Boot loader configuration (for physical/VM installations)
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # Minimal filesystem configuration - adjust based on actual system
  fileSystems."/" = {
    device = "/dev/disk/by-label/nixos";
    fsType = "ext4";
  };

  fileSystems."/boot" = {
    device = "/dev/disk/by-label/boot";
    fsType = "vfat";
    options = [ "fmask=0077" "dmask=0077" ];
  };

  # Enable Docker
  virtualisation.docker = {
    enable = true;
  };

  # Fonts
  fonts.packages = with pkgs; [
    noto-fonts-cjk-sans
    noto-fonts-color-emoji
  ];

  # Shell configuration
  programs.zsh.enable = true;
  environment.pathsToLink = [ "/share/zsh" ];
  environment.shells = [ pkgs.zsh ];

  # User configuration
  users.users.ktaga = {
    isNormalUser = true;
    shell = pkgs.zsh;
    extraGroups = [
      "wheel"
      "docker"
      "networkmanager"
    ];
  };

  # Networking
  networking.networkmanager.enable = true;

  # OpenSSH
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "no";
      PasswordAuthentication = false;
    };
  };

  # System state version
  system.stateVersion = "24.11";
}
