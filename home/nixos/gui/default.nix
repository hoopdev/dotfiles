{ pkgs, ... }:
{
  imports = [
    ./browser.nix
    ./fcitx5.nix
    ./hyprland.nix
    ./hyprlock.nix
    ./hypridle.nix
    ./wlogout.nix
    ./wofi.nix
    ./gtk.nix
    ./swayosd.nix
  ];

  home.packages = with pkgs; [
    obsidian
  ];
}
