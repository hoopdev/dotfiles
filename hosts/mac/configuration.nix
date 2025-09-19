{ pkgs, ... }:
{
  # List packages installed in system profile. To search by name, run:
  # $ nix-env -qaP | grep wget

  services = {
    aerospace = {
      enable = true;
      settings = builtins.fromTOML (builtins.readFile ../../.aerospace.toml);
    };
  };

  # Homebrew設定
  homebrew = {
    enable = true;
    masApps = {
      # WindowsApp = 1295203466;
      # Line = 539883307;
    };
    brews = [
      "ffmpeg"
      "rsync"
      "libiconv"
      "node"
    ];
    casks = [
      "vivaldi"
      "1password"
      "notion"
      "orbstack"
      "parsec"
      "microsoft-office"
      "microsoft-auto-update"
      "raycast"
      "ngrok"
      "karabiner-elements"
      "google-drive"
      "chatgpt"
      "google-japanese-ime"
      "figma"
      "blender"
      "tailscale"
      "bambu-studio"
      "ollama"
      "claude"
      "signal"
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


  # DS_Store作成防止設定
  system.defaults.CustomUserPreferences = {
    "com.apple.desktopservices" = {
      # ネットワークボリュームでDS_Storeを作成しない
      DSDontWriteNetworkStores = true;
      # USBボリュームでDS_Storeを作成しない
      DSDontWriteUSBStores = true;
    };
  };

  # キーボード設定 (CapsLockのremapはkarabiner-elementsで実行)
  system.keyboard = {
    enableKeyMapping = true;
  };

  # Touch ID for sudo
  security.pam.services.sudo_local.touchIdAuth = true;

  # Auto upgrade nix package and the daemon service.
  nix.package = pkgs.nix;

  # Necessary for using flakes on this system.
  nix.settings.experimental-features = "nix-command flakes";

  # Used for backwards compatibility, please read the changelog before changing.
  # $ darwin-rebuild changelog
  system.stateVersion = 5;

  # Set primary user for nix-darwin migration
  system.primaryUser = "ktaga";

  # Allow unfree
  nixpkgs.config.allowUnfree = true;

  # The platform the configuration will be used on.
  nixpkgs.hostPlatform = "aarch64-darwin";
}
