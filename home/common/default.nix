{ lib, pkgs, username, inputs, ... }:

{
  imports = [
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

  colorscheme = inputs.nix-colors.colorSchemes.nord;
}
