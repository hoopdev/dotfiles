{ lib, pkgs, username, inputs, ... }:

{
  imports = [
    ../../home/common/cli
    inputs.nixvim.homeModules.nixvim
    ./starship.nix  # Ubuntu-specific Starship configuration
  ];

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

  # Create ~/.zshrc to source home-manager's zsh configuration
  home.file.".zshrc".text = ''
    # Source home-manager's zsh configuration
    export ZDOTDIR="''${XDG_CONFIG_HOME:-$HOME/.config}/zsh"
    if [ -f "$ZDOTDIR/.zshrc" ]; then
      source "$ZDOTDIR/.zshrc"
    fi

    # Source local environment if it exists
    if [ -f "$HOME/.local/bin/env" ]; then
      . "$HOME/.local/bin/env"
    fi

    # Source Nix profile if it exists
    if [ -e "$HOME/.nix-profile/etc/profile.d/nix.sh" ]; then
      . "$HOME/.nix-profile/etc/profile.d/nix.sh"
    fi
  '';
}
