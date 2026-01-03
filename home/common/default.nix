{ lib, pkgs, username, inputs, ... }:

{
  imports = [
    # nix-colors disabled - using Stylix for theming instead
    # inputs.nix-colors.homeManagerModules.default
    inputs.nixvim.homeModules.nixvim
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

  # nix-colors disabled - Stylix handles Nord theming via lib/stylix.nix
  # colorScheme = inputs.nix-colors.colorSchemes.nord;
}
