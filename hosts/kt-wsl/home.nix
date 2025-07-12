{ lib, pkgs, username, inputs, ... }:

{
  imports = [
    ../../home/common/cli
    inputs.nix-colors.homeManagerModules.default
  ];

  colorScheme = inputs.nix-colors.colorSchemes.nord;

  home = {
    inherit username;
    homeDirectory = "/home/${username}";
    stateVersion = "24.05";
  };

  home.packages = [
    inputs.nixvim.packages.x86_64-linux.default
  ];
}