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
    onActivation = {
      autoUpdate = true; # brew update を自動実行
      upgrade = true; # brew upgrade を自動実行
      cleanup = "zap"; # 未使用パッケージを削除 (zap: caskの設定ファイルも削除)
    };
    masApps = {
      # WindowsApp = 1295203466;
      # Line = 539883307;
    };
    taps = [
      "coder/coder"
    ];
    brews = [
      "coder"
      "displayplacer"
      "ffmpeg"
      "rsync"
      "libiconv"
      "node"
    ];
    casks = [
      "claude-code"
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
      "tailscale-app"
      "bambu-studio"
      "ollama-app"
      "claude"
      "signal"
      "alt-tab"
      "discord"
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
