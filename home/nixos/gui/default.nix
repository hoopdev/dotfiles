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

  # GUI apps from nixpkgs. These are deliberately NOT in home/common/gui:
  # macOS takes the same apps from Homebrew casks instead, because nixpkgs'
  # darwin builds are DMG/zip repacks whose .app bundles macOS App Management
  # later refuses to let Nix delete (fchmodat EPERM), wedging every rebuild.
  home.packages = with pkgs; [
    obsidian
    slack
    vscode
  ];
}
