{
  lib,
  pkgs,
  config,
  ...
}:

{
  # Enable nix-ld for running unpatched dynamic binaries (e.g., uv, Python wheels)
  programs.nix-ld = {
    enable = true;
    libraries = with pkgs; [
      # Core libraries
      stdenv.cc.cc.lib
      zlib
      glib
      libffi
      openssl

      # Compression
      xz
      bzip2
      zstd

      # Terminal/CLI
      ncurses
      readline

      # Database
      sqlite

      # Graphics/Fonts (for GUI apps)
      freetype
      fontconfig
      expat
      libGL
      xorg.libX11
      xorg.libXext
      xorg.libXrender
      xorg.libICE
      xorg.libSM
      xorg.libXcursor
      xorg.libXrandr
      xorg.libXi

      # Misc
      libxkbcommon
      dbus
    ];
  };

  # 1Password
  programs._1password.enable = true;
  programs._1password-gui = {
    enable = true;
    polkitPolicyOwners = [ "ktaga" ];
  };

  # Common Nix settings for all NixOS hosts
  nix = {
    settings = {
      auto-optimise-store = true;
      experimental-features = [
        "nix-command"
        "flakes"
      ];
      # Hyprland cache
      substituters = [ "https://hyprland.cachix.org" ];
      trusted-public-keys = [
        "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
      ];
    };
  };

  # Allow unfree packages
  nixpkgs.config.allowUnfree = true;
}
