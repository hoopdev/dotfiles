{ pkgs, lib, username, inputs, ... }:
{
  imports = [
    ../../home/common/cli
    inputs.nix-colors.homeManagerModules.default
  ];

  home = {
    inherit username;
    homeDirectory = "/home/${username}";
    stateVersion = "24.05";
  };

  programs.home-manager.enable = true;
  colorScheme = inputs.nix-colors.colorSchemes.nord;

  home.packages = [
    inputs.nixvim.packages.x86_64-linux.default
  ];
}
