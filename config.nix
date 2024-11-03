{ pkgs, ... }:
{
  # List packages installed in system profile. To search by name, run:
  # $ nix-env -qaP | grep wget

  services.aerospace = {
    enable = true;
    settings = builtins.fromTOML (builtins.readFile ./starship.toml);
    # settings = {
    #   accordion-padding = 30;
    #   after-login-command = [ ];
    #   after-startup-command = [ ];
    #   default-root-container-layout = "tiles";
    #   default-root-container-orientation = "auto";
    #   enable-normalization-flatten-containers = true;
    #   enable-normalization-opposite-orientation-for-nested-containers = true;
    #   exec-on-workspace-change = [ ];
    #   on-focus-changed = [ ];
    #   on-focused-monitor-changed = [ "move-mouse monitor-lazy-center" ];
    #   on-window-detected = [ ];
    #   key-mapping = {
    #     preset = "qwerty";
    #   };
    #   gaps = {
    #     inner.horizontal = 0;
    #     inner.vertical = 0;
    #     outer.left = 0;
    #     outer.bottom = 0;
    #     outer.top = 0;
    #     outer.right = 0;
    #   };

    #   mode.main.binding = {
    #     alt-slash = "layout tiles horizontal vertical";
    #     alt-comma = "layout accordion horizontal vertical";

    #     alt-h = "focus left";
    #     alt-j = "focus down";
    #     alt-k = "focus up";
    #     alt-l = "focus right";

    #     alt-shift-h = "move left";
    #     alt-shift-j = "move down";
    #     alt-shift-k = "move up";
    #     alt-shift-l = "move right";

    #     alt-shift-minus = "resize smart -50";
    #     alt-shift-equal = "resize smart +50";

    #     alt-1 = "workspace 1";
    #     alt-2 = "workspace 2";
    #     alt-3 = "workspace 3";
    #     alt-4 = "workspace 4";
    #     alt-5 = "workspace 5";
    #     alt-a = "focus-monitor left";
    #     alt-d = "focus-monitor right";
    #     alt-f = "fullscreen";
    #     alt-m = "macos-native-minimize";
    #     alt-s = "focus-monitor up";
    #     alt-w = "focus-monitor down";

    #     alt-shift-1 = "move-node-to-workspace 1";
    #     alt-shift-2 = "move-node-to-workspace 2";
    #     alt-shift-3 = "move-node-to-workspace 3";
    #     alt-shift-4 = "move-node-to-workspace 4";
    #     alt-shift-5 = "move-node-to-workspace 5";
    #     alt-shift-a = "move-node-to-monitor left";
    #     alt-shift-d = "move-node-to-monitor right";
    #     alt-shift-f = "macos-native-fullscreen";
    #     alt-shift-s = "move-node-to-monitor up";
    #     alt-shift-w = "move-node-to-monitor down";

    #     alt-tab = "workspace-back-and-forth";
    #     alt-shift-tab = "move-workspace-to-monitor --wrap-around next";
    #     alt-shift-semicolon = "mode service";
    #   };
    # };

  };

  # Homebrew設定
  homebrew = {
    enable = true;
    masApps = {
      Tailscale = 1475387142;
      WindowsApp = 1295203466;
      Line = 539883307;
      Perplexity = 6714467650;
    };
    brews =
      [
      ];
    casks = [
      "vivaldi"
      "warp"
      "notion"
      "orbstack"
      "parsec"
      "dropbox"
      "microsoft-office"
      "microsoft-auto-update"
      "raycast"
      "ngrok"
      "karabiner-elements"
      "clipy"
      "arduino"
      "google-drive"
      "chatgpt"
    ];
  };

  # Finder設定
  system.defaults.finder = {
    AppleShowAllExtensions = true;
    AppleShowAllFiles = true;
    CreateDesktop = false;
    FXEnableExtensionChangeWarning = false;
    ShowPathbar = true;
    ShowStatusBar = true;
  };

  # Dock設定
  system.defaults.dock = {
    autohide = true;
    show-recents = false;
    tilesize = 50;
    magnification = true;
    largesize = 64;
    orientation = "left";
    mineffect = "scale";
    launchanim = true;
  };

  # Auto upgrade nix package and the daemon service.
  services.nix-daemon.enable = true;
  nix.package = pkgs.nix;

  # Necessary for using flakes on this system.
  nix.settings.experimental-features = "nix-command flakes";

  # Used for backwards compatibility, please read the changelog before changing.
  # $ darwin-rebuild changelog
  system.stateVersion = 5;

  # Allow unfree
  nixpkgs.config.allowUnfree = true;

  # The platform the configuration will be used on.
  nixpkgs.hostPlatform = "aarch64-darwin";
}
