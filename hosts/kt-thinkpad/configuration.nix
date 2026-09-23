# Edit this configuration file to define what should be installed on
# your system.  Help is available in the configuration.nix(5) man page
# and in the NixOS manual (accessible by running ‘nixos-help’).

{
  inputs,
  lib,
  pkgs,
  primaryUser,
  ...
}:

{
  imports = [
    # Include the results of the hardware scan.
    ./hardware-configuration.nix
    inputs.xremap.nixosModules.default
    ../../lib/japanese-locale.nix
    ../../lib/japanese-input.nix
    ((import ../../lib/users.nix).mkUser {
      username = primaryUser;
      extraGroups = [
        "networkmanager"
        "audio"
        "video"
        "input"
      ];
    })
  ]
  ++ (with inputs.nixos-hardware.nixosModules; [
    lenovo-thinkpad
    common-cpu-intel
    common-pc-ssd
  ]);

  boot = {
    loader = {
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
    consoleLogLevel = 0;
    initrd.verbose = false;
    kernelParams = [
      "quiet"
      "udev.log_level=3"
    ];
  };

  # HiDPI panel: lib/stylix.nix defaults the cursor to 24px, which is too
  # small at 2x. Propagates to Home Manager (XCURSOR_SIZE, GTK, Hyprland).
  stylix.cursor.size = lib.mkForce 32;

  hardware = {
    # TrackPoint support for keyboard's TrackPoint via RMI4
    trackpoint.enable = true;
    bluetooth = {
      enable = true;
      powerOnBoot = true;
    };
  };

  networking.networkmanager.enable = true;

  services = {
    xremap = {
      enable = true;
      userName = primaryUser;
      serviceMode = "system";
      watch = true;
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
        keymap = [ ];
      };
    };

    # udev rule to bind psmouse to RMI4 PS/2 pass-through (TrackPoint on keyboard)
    udev.extraRules = ''
      ACTION=="add", SUBSYSTEM=="serio", ATTR{description}=="RMI4 PS/2 pass-through", ATTR{drvctl}="psmouse"
    '';

    # Enable X11 for XWayland support
    xserver = {
      enable = true;
      xkb = {
        layout = "us";
        variant = "";
      };
    };

    # Enable libinput for touchpad/trackpoint
    libinput = {
      enable = true;
      touchpad = {
        naturalScrolling = true;
        tapping = true;
        clickMethod = "clickfinger";
      };
    };

    # greetd + tuigreet for TUI login
    greetd = {
      enable = true;
      settings = {
        default_session = {
          command = "${pkgs.tuigreet}/bin/tuigreet --time --remember --cmd start-hyprland";
          user = "greeter";
        };
      };
    };

    # Enable CUPS to print documents.
    printing.enable = true;

    # Enable sound with pipewire.
    pulseaudio.enable = false;
    pipewire = {
      enable = true;
      alsa.enable = true;
      alsa.support32Bit = true;
      pulse.enable = true;
    };

    tailscale.enable = true;

    # UPower for battery monitoring (used by the Wayle battery module)
    upower.enable = true;

    openssh.enable = true;
  };

  security.rtkit.enable = true;

  # Suppress getty on tty1 since we use greetd
  systemd.services = {
    "getty@tty1".enable = false;
    "autovt@tty1".enable = false;
  };

  # User account: skeleton (isNormalUser, wheel, zsh) comes from lib/users.nix,
  # imported above; only the host-specific groups are listed there.

  programs.hyprland = {
    enable = true;
    xwayland.enable = true;
  };

  # xdg-desktop-portal for screen sharing and dark mode detection
  xdg.portal = {
    enable = true;
    extraPortals = [
      pkgs.xdg-desktop-portal-gtk
      pkgs.xdg-desktop-portal-hyprland
    ];
  };

  # This value determines the NixOS release from which the default
  # settings for stateful data, like file locations and database versions
  # on your system were taken. It's perfectly fine and recommended to leave
  # this value at the release version of the first install of this system.
  # Before changing this value read the documentation for this option
  # (e.g. man configuration.nix or on https://nixos.org/nixos/options.html).
  system.stateVersion = "24.05"; # Did you read the comment?
}
