{ pkgs, ... }:
{
  home.packages = with pkgs; [
    obsidian
    slack
    vscode
    zed-editor
    zoom-us
    signal-desktop
    ollama
    xld
    discord
  ];
}
