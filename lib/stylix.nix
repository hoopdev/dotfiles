# Unified Stylix theming for NixOS, nix-darwin, and home-manager.
#
# Pass `darwin = true` from `mkDarwinConfiguration` so the cursor option is
# omitted (nix-darwin's stylix module doesn't expose `stylix.cursor`). All
# other contexts (NixOS, standalone home-manager on Linux) leave it default.
{
  darwin ? false,
}:
{ pkgs, lib, ... }:

{
  stylix = {
    enable = true;

    # Stylix's `release` constant (26.05) trails home-manager's (26.11), but both
    # track nixpkgs-unstable so they're compatible. Skip the mismatch warning.
    enableReleaseChecks = false;

    image = ../wallpaper/wallpaper_enoshima.jpg;
    base16Scheme = ./shonan.yaml;
    polarity = "dark";

    fonts = {
      serif = {
        package = pkgs.noto-fonts-cjk-serif;
        name = "Noto Serif CJK JP";
      };
      sansSerif = {
        package = pkgs.noto-fonts-cjk-sans;
        name = "Noto Sans CJK JP";
      };
      # HackGen rather than plain Hack: it is the monospace font that is also
      # easy to install on Windows, so the exported (non-Nix) config can name
      # the same font. Stylix's font-packages target installs it on every
      # platform, so it needs no separate entry in home.packages / fonts.packages.
      monospace = {
        package = pkgs.hackgen-nf-font;
        name = "HackGen Console NF";
      };
      emoji = {
        package = pkgs.noto-fonts-color-emoji;
        name = "Noto Color Emoji";
      };
      sizes = {
        terminal = 14;
        applications = 12;
        desktop = 12;
        popups = 12;
      };
    };

    opacity = {
      # 0.9, not 0.95: this is the value WezTerm actually ran with, back when
      # its extraConfig overrode Stylix. Keeping it here preserves the look now
      # that the override is gone.
      terminal = 0.9;
      desktop = 1.0;
      popups = 0.95;
      applications = 1.0;
    };
  }
  // lib.optionalAttrs (!darwin) {
    cursor = {
      package = pkgs.nordzy-cursor-theme;
      name = "Nordzy-cursors";
      size = 24;
    };
  };
}
