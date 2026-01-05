# Stylix theming configuration
# This module provides a unified theming system across all platforms
{ pkgs, ... }:

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

    # Cursor configuration
    cursor = {
      package = pkgs.nordzy-cursor-theme;
      name = "Nordzy-cursors";
      size = 24;
    };

    # Opacity settings
    opacity = {
      terminal = 0.95;
      desktop = 1.0;
      popups = 0.95;
      applications = 1.0;
    };
  };
}
