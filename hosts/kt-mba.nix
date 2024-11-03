{ pkgs, ... }:
{
  imports = [
    ../../home-manager/cli
    ../../home-manager/gui
  ];

  home.packages =
    with pkgs;
    [
    ];
}
