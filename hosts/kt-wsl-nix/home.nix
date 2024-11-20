{
  pkgs,
  lib,
  username,
  inputs,
  ...
}:
{
  home = rec {
    inherit username;
    homeDirectory = "/home/${username}";
    stateVersion = "24.05";
  };

  programs.home-manager.enable = true;
  #programs.kitty.enable = true;
  imports = [
    ../../home/common/cli
    inputs.nix-colors.homeManagerModules.default
    #inputs.nixvim.homeManagerModules.nixvim
  ];
  colorScheme = inputs.nix-colors.colorSchemes.nord;

  home.packages = [
    inputs.nixvim.packages.x86_64-linux.default
  ];
}
