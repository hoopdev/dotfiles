{ pkgs, ... }: {
    # List packages installed in system profile. To search by name, run:
    # $ nix-env -qaP | grep wget
    environment.systemPackages =
    [
    	pkgs.arc-browser
    	pkgs.discord
    	pkgs.slack
    	pkgs.zoom-us
    	pkgs.signal-desktop
	pkgs.hackgen-nf-font
    	pkgs.uv
    	pkgs.deno
    	pkgs.zsh-autocomplete
    	pkgs.zsh-autosuggestions
    	pkgs.zsh-autopair
    	pkgs.ollama
    	pkgs.obsidian
    	pkgs.docker
    	pkgs.xld
    ];

    # Homebrew設定
    homebrew = {
        enable = true;
	masApps = 
	{
	Tailscale = 1475387142;
	};
	brews =
	[
	];
	casks =
	    [
	    "aerospace"
	    "orbstack"
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
	    "font-hackgen-nerd"
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

    # Enable alternative shell support in nix-darwin.
    programs.zsh.enable = true;
    # programs.fish.enable = true;

    # Used for backwards compatibility, please read the changelog before changing.
    # $ darwin-rebuild changelog
    system.stateVersion = 5;

    # Allow unfree
    nixpkgs.config.allowUnfree = true;

    # The platform the configuration will be used on.
    nixpkgs.hostPlatform = "aarch64-darwin";
}
