{ pkgs, ... }:
{
  home.packages = with pkgs; [
    obsidian
    slack
    telegram-desktop
    zed-editor
    vscode
  ];
}
