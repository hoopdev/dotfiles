{ pkgs, ... }:
{
  home.packages = with pkgs; [
    zoom-us
    signal-desktop
    xld
    discord
  ];
}
