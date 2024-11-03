{ pkgs, ... }:
{
  imports = [
    ../../home/cli
    ../../home/gui
  ];

  home.packages =
    with pkgs;
    [
    ];
}
