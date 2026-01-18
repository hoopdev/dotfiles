# Edit this configuration file to define what should be installed on
# your system.  Help is available in the configuration.nix(5) man page
# and in the NixOS manual (accessible by running ‘nixos-help’).

{ inputs, pkgs, ... }:

{
  imports = [
    # Include the results of the hardware scan.
    ./hardware-configuration.nix
    inputs.xremap.nixosModules.default
    ../../lib/nixos-common.nix
    ../../lib/japanese-locale.nix
  ]
  ++ (with inputs.nixos-hardware.nixosModules; [
    lenovo-thinkpad
    common-cpu-intel
    common-pc-ssd
  ]);

  services.xremap = {
    enable = true;
    userName = "ktaga";
    serviceMode = "system";
    config = {
      modmap = [
        {
          # CapsLockをCtrlに置換
          name = "Caps2Ctrl";
          remap = {
            CapsLock = "Ctrl_L";
          };
        }
      ];
      keymap = [
      ];
    };
  };

  #Bootloader.
  boot.loader = {
    timeout = 2;
    efi = {
      canTouchEfiVariables = true;
      efiSysMountPoint = "/boot";
    };
    grub = {
      enable = true;
      efiSupport = true;
      device = "nodev";
    };
  };

  # Reduce console log level to prevent logs from appearing on login screen
  boot.consoleLogLevel = 0;
  boot.initrd.verbose = false;
  boot.kernelParams = [
    "quiet"
    "udev.log_level=3"
  ];

  # TrackPoint support for keyboard's TrackPoint via RMI4
  hardware.trackpoint.enable = true;

  # udev rule to bind psmouse to RMI4 PS/2 pass-through (TrackPoint on keyboard)
  services.udev.extraRules = ''
    ACTION=="add", SUBSYSTEM=="serio", ATTR{description}=="RMI4 PS/2 pass-through", ATTR{drvctl}="psmouse"
  '';

  # Enable networking
  networking.networkmanager.enable = true;

  # Enable X11 for XWayland support
  services.xserver.enable = true;

  # Enable libinput for touchpad/trackpoint
  services.libinput = {
    enable = true;
    touchpad = {
      naturalScrolling = true;
      tapping = true;
      clickMethod = "clickfinger";
    };
  };

  # greetd + tuigreet for TUI login
  services.greetd = {
    enable = true;
    settings = {
      default_session = {
        command = "${pkgs.tuigreet}/bin/tuigreet --time --remember --cmd Hyprland";
        user = "greeter";
      };
    };
  };

  # Suppress getty on tty1 since we use greetd
  systemd.services."getty@tty1".enable = false;
  systemd.services."autovt@tty1".enable = false;

  # Configure keymap in X11
  services.xserver.xkb = {
    layout = "us";
    variant = "";
  };

  # Enable CUPS to print documents.
  services.printing.enable = true;

  # Enable sound with pipewire.
  services.pulseaudio.enable = false;
  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
  };

  services.tailscale.enable = true;

  # Bluetooth support
  hardware.bluetooth = {
    enable = true;
    powerOnBoot = true;
  };

  # UPower for battery monitoring (required by HyprPanel)
  services.upower.enable = true;

  # Define a user account. Don't forget to set a password with ‘passwd’.
  users.users.ktaga = {
    isNormalUser = true;
    description = "ktaga";
    extraGroups = [
      "networkmanager"
      "wheel"
      "audio"
      "video"
      "input"
    ];
    shell = pkgs.zsh;
    packages = with pkgs; [
      zsh
    ];
  };

  # Install programs
  programs = {
    zsh.enable = true;
    hyprland = {
      enable = true;
      xwayland.enable = true;
    };
  };

  # xdg-desktop-portal for screen sharing and dark mode detection
  xdg.portal = {
    enable = true;
    extraPortals = [
      pkgs.xdg-desktop-portal-gtk
      pkgs.xdg-desktop-portal-hyprland
    ];
  };

  environment.systemPackages = with pkgs; [ ];
  environment.variables = { };

  services.openssh.enable = true;

  # This value determines the NixOS release from which the default
  # settings for stateful data, like file locations and database versions
  # on your system were taken. It's perfectly fine and recommended to leave
  # this value at the release version of the first install of this system.
  # Before changing this value read the documentation for this option
  # (e.g. man configuration.nix or on https://nixos.org/nixos/options.html).
  system.stateVersion = "24.05"; # Did you read the comment?
}
