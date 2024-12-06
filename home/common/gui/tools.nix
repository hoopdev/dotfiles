{ pkgs, ... }:
{
  home.packages = with pkgs; [
    obsidian
    slack
    zed-editor
  ];
}
