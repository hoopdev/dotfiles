{ lib, config, pkgs, username, inputs, ... }:
{
  imports = [
    ./cli
    ./gui
  ];

  # Common home settings
  home = {
    inherit username;
    stateVersion = "24.05";

    sessionVariables = {
      EDITOR = "nvim";
      NIXPKGS_ALLOW_UNFREE = 1;
    };
  };

  # Enable home-manager
  programs.home-manager.enable = true;

  # Common color scheme
  colorScheme = inputs.nix-colors.colorSchemes.nord;
}
