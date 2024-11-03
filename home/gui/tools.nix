{ pkgs, ... }:
{
  home.packages = with pkgs; [
    obsidian
    parsec-bin
    slack
    vscode
    zoom-us
  ];
}