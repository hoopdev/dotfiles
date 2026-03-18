{ pkgs, ... }:
{
  home.packages = with pkgs; [
    slack
    telegram-desktop
    zed-editor
    vscode
  ];
}
