{ pkgs, lib, username, inputs, ... }:
{
  imports = [
    ../../home/mac
    inputs.nix-colors.homeManagerModules.default
  ];

  home = {
    homeDirectory = "/Users/${username}";
  };

  home.packages = with pkgs; [
    inputs.nixvim.packages.aarch64-darwin.default
  ];
}
