{ pkgs, ... }:
{
  home.packages = with pkgs; [
    zoom-us
    signal-desktop
    ollama
    xld
    discord
    vscode
  ];
}
