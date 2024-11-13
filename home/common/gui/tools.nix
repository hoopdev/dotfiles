{ pkgs, ... }:
{
  home.packages = with pkgs; [
    obsidian
    slack
    vscode
    zed-editor
    ollama
  ];
}
