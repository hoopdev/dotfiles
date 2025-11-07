{ lib, pkgs, username, inputs, ... }:

{
  imports = [
    ../../home/common/cli
    inputs.nix-colors.homeManagerModules.default
    inputs.nixvim.homeModules.nixvim
  ];

  colorScheme = inputs.nix-colors.colorSchemes.nord;

  home = {
    inherit username;
    homeDirectory = "/home/${username}";
    stateVersion = "24.05";
  };
}
