{ pkgs, ... }:
{
  home.packages = with pkgs; [
    arc-browser
  ];
  programs =
    {
    };
}
