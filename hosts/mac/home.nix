{
  pkgs,
  lib,
  username,
  inputs,
  ...
}:
{
  # ユーザー情報
  home.username = username;
  home.homeDirectory = "/Users/${username}";

  # home-managerのバージョン
  home.stateVersion = "24.05";

  # home-managerの有効化
  programs.home-manager.enable = true;

  imports = [
    ../../home/common/cli
    ../../home/common/gui
    ../../home/mac/gui
    inputs.nix-colors.homeManagerModules.default
    #inputs.nixvim.homeManagerModules.nixvim
  ];
  colorScheme = inputs.nix-colors.colorSchemes.nord;

  home.packages =
    with pkgs;
    [
      inputs.nixvim.packages.aarch64-darwin.default
    ];
}
