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

  # Bootloader.
  #boot.loader.grub.enable = true;
  #boot.loader.grub.device = "/dev/sda";
  #boot.loader.grub.useOSProber = true;
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = false;

  networking.hostName = "kt-proxmox";

  # Enable networking
  networking.networkmanager.enable = true;

  # Additional Japanese locale settings for this host
  i18n.extraLocaleSettings = {
    LC_ADDRESS = "ja_JP.UTF-8";
    LC_IDENTIFICATION = "ja_JP.UTF-8";
    LC_MEASUREMENT = "ja_JP.UTF-8";
    LC_MONETARY = "ja_JP.UTF-8";
    LC_NAME = "ja_JP.UTF-8";
    LC_NUMERIC = "ja_JP.UTF-8";
    LC_PAPER = "ja_JP.UTF-8";
    LC_TELEPHONE = "ja_JP.UTF-8";
    LC_TIME = "ja_JP.UTF-8";
  };

  # Hardware graphics
  hardware.graphics = {
    enable = true;
  };

  # Desktop services are disabled by the headless system profile.
  # Keep CLI integration without installing the 1Password GUI on this host.
  programs._1password.enable = true;

  # User account: skeleton (isNormalUser, wheel, zsh) comes from lib/users.nix,
  # imported above; only the host-specific groups are listed there.

  # List packages installed in system profile. To search, run:
  # $ nix search wget
  environment.systemPackages = with pkgs; [
    devenv
    git-lfs
    google-cloud-sdk
  ];

  # Some programs need SUID wrappers, can be configured further or are
  # started in user sessions.
  # programs.mtr.enable = true;
  # programs.gnupg.agent = {
  #   enable = true;
  #   enableSSHSupport = true;
  # };

  # List services that you want to enable:

  # Enable the OpenSSH daemon.
  services.openssh.enable = true;

  services.tailscale.enable = true;
  services.ollama = {
    enable = true;
    # TODO: package = pkgs.ollama-cuda; — enable once GPU recognition is confirmed
    host = "0.0.0.0";
    port = 11434;
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
  nix.settings = {
    trusted-users = [
      "root"
      primaryUser
    ];
  };
}
