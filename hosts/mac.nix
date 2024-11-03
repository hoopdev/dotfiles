{ pkgs, lib, username, ... }:
{
  # ユーザー情報
  home.username = username;
  home.homeDirectory = "/Users/${username}";

  # home-managerのバージョン
  home.stateVersion = "24.05";

  # home-managerの有効化
  programs.home-manager.enable = true;

  imports = [
    ../home/cli
    ../home/gui
  ];

  home.packages =
    with pkgs;
    [
    ];
}
