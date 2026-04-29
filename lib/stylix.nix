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
      monospace = {
        package = pkgs.nerd-fonts.hack;
        name = "Hack Nerd Font";
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
      terminal = 0.95;
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
