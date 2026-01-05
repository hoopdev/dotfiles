# Stylix system-level theming configuration for Darwin (macOS)
{ pkgs, lib, ... }:

{
  stylix = {
    enable = true;

    # Wallpaper image (required by Stylix)
    image = ../wallpaper/wallpaper_enoshima.jpg;

    # Use Shonan color scheme (custom blend of Nord and Tokyo Night)
    base16Scheme = ./shonan.yaml;

    # Dark theme polarity
    polarity = "dark";

    # Font configuration
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

    # Opacity settings
    opacity = {
      terminal = 0.95;
      desktop = 1.0;
      popups = 0.95;
      applications = 1.0;
    };

    # Note: cursor is not configured on Darwin (option doesn't exist)
  };
}
