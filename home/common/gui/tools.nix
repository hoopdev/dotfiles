{ pkgs, ... }:
{
  home.packages = with pkgs; [
    slack
    telegram-desktop
    vscode
  ];
}
