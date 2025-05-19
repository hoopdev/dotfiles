{ pkgs, ... }:
{
  home.packages = with pkgs; [
    zoom-us
    xld
    discord
  ];
}
