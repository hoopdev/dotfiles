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
      cleanup = "none"; # 手動 brew install を消さない (以前は "zap" で ollama 等が毎回消えていた)
    };
    masApps = {
      # WindowsApp = 1295203466;
      # Line = 539883307;
    };
    taps = [
      "coder/coder"
      "oven-sh/bun"
    ];
    brews = [
      "coder"
      "displayplacer"
      "ffmpeg"
      "opencode"
      "rsync"
      "libiconv"
      "node"
      "oven-sh/bun/bun"
      "syncthing"
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
      "bambu-studio"
      "claude"
      "signal"
      "alt-tab"
      "discord"
      "obsidian"
      "telegram"
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

  # nixos-render-docs on nixpkgs unstable dropped --toc-depth, which the
  # pinned nix-darwin still passes when building the HTML manual.
  # Re-enable once nix-darwin/nix-darwin#1819 is merged and the input bumped.
  documentation.doc.enable = false;

  # Auto upgrade nix package and the daemon service.
  nix.package = pkgs.nix;

  # Necessary for using flakes on this system.
  nix.settings.experimental-features = "nix-command flakes";

  # Used for backwards compatibility, please read the changelog before changing.
  # $ darwin-rebuild changelog
  system.stateVersion = 5;

}
