{ pkgs, ... }:
{
  home.packages = with pkgs; [
    vivaldi
  ];
  programs =
    {
    };
}
