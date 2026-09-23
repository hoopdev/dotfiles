# Edit this configuration file to define what should be installed on
# your system.  Help is available in the configuration.nix(5) man page
# and in the NixOS manual (accessible by running ‘nixos-help’).

{
  pkgs,
  primaryUser,
  ...
}:

{
  imports = [
    # Include the results of the hardware scan.
    ./hardware-configuration.nix
    ../../lib/japanese-locale.nix
    ((import ../../lib/users.nix).mkUser {
      username = primaryUser;
      extraGroups = [
        "networkmanager"
        "audio"
        "video"
        "docker"
      ];
    })
  ];

  boot.loader = {
    systemd-boot.enable = true;
    efi.canTouchEfiVariables = false;
  };

  networking = {
    hostName = "kt-proxmox";
    networkmanager.enable = true;
    # Ollama is reachable over Tailscale only; the LAN side stays closed.
    firewall.interfaces.tailscale0.allowedTCPPorts = [ 11434 ];
  };

  # Hardware graphics
  hardware.graphics.enable = true;

  # Desktop services are disabled by the headless system profile.
  # Keep CLI integration without installing the 1Password GUI on this host.
  programs._1password.enable = true;

  # User account: skeleton (isNormalUser, wheel, zsh) comes from lib/users.nix,
  # imported above; only the host-specific groups are listed there.

  environment.systemPackages = with pkgs; [
    devenv
    git-lfs
    google-cloud-sdk
  ];

  services = {
    openssh.enable = true;
    tailscale.enable = true;
    ollama = {
      enable = true;
      # TODO: package = pkgs.ollama-cuda; — enable once GPU recognition is confirmed
      host = "0.0.0.0";
      port = 11434;
    };
  };

  virtualisation.docker = {
    enable = true;
    enableOnBoot = true;
    extraPackages = with pkgs; [ docker-buildx ];
  };

  # This value determines the NixOS release from which the default
  # settings for stateful data, like file locations and database versions
  # on your system were taken. It's perfectly fine and recommended to leave
  # this value at the release version of the first install of this system.
  # Before changing this value read the documentation for this option
  # (e.g. man configuration.nix or on https://nixos.org/nixos/options.html).
  system.stateVersion = "24.05"; # Did you read the comment?

  # Host-specific Nix settings (extends modules/nixos/nix-settings.nix)
  nix.settings.trusted-users = [
    "root"
    primaryUser
  ];
}
