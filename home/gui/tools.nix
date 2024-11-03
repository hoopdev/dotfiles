{ pkgs, ... }:
{
  home.packages = with pkgs; [
    obsidian
    slack
    vscode
    zoom-us
    signal-desktop
    ollama
    xld
  ];
}
