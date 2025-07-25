{ lib, pkgs, username, inputs, ... }:

{
  imports = [
    inputs.nix-colors.homeManagerModules.default
    ./cli
    ./gui
  ];

  home = {
    inherit username;
    stateVersion = "24.05";

    sessionVariables = {
      EDITOR = "nvim";
      NIXPKGS_ALLOW_UNFREE = "1";
    };
  };

  programs.home-manager.enable = true;

  colorScheme = inputs.nix-colors.colorSchemes.nord;
}
