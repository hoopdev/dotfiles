{ lib, pkgs, username, inputs, ... }:

{
  imports = [
    ../../home/common/cli
    inputs.nix-colors.homeManagerModules.default
    inputs.nixvim.homeModules.nixvim
    ./starship.nix  # Ubuntu-specific Starship configuration
  ];

  colorScheme = inputs.nix-colors.colorSchemes.nord;

  home = {
    inherit username;
    homeDirectory = "/home/${username}";
    stateVersion = "24.05";

    # Fonts for icons and emoji support in terminal
    packages = with pkgs; [
      noto-fonts-cjk-sans
      noto-fonts-color-emoji
      # Nerd Fonts for Starship icons and Ubuntu logo
      (nerd-fonts.fira-code)
      (nerd-fonts.jetbrains-mono)
      (nerd-fonts.meslo-lg)
    ];
  };

  # Enable font configuration in home-manager
  fonts.fontconfig.enable = true;

  programs.home-manager.enable = true;
}
