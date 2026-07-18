{
  pkgs,
  ...
}:

{
  home.packages = with pkgs; [
    noto-fonts-cjk-sans
    noto-fonts-color-emoji
    # Nerd Fonts for Starship icons and Ubuntu logo
    nerd-fonts.fira-code
    nerd-fonts.jetbrains-mono
    nerd-fonts.meslo-lg
  ];

  # Enable font configuration in home-manager
  fonts.fontconfig.enable = true;

  programs.home-manager.enable = true;

  # Ubuntu logo in the starship system segment (overrides the common default).
  programs.starship.systemLogo = "󰕈";

  # Disable dconf - not needed in Docker/container environments
  dconf.enable = false;

  # Create ~/.zshrc to source home-manager's zsh configuration
  home.file.".zshrc".text = ''
    # Source Home Manager session variables before zsh init uses them
    if [ -f "$HOME/.nix-profile/etc/profile.d/hm-session-vars.sh" ]; then
      . "$HOME/.nix-profile/etc/profile.d/hm-session-vars.sh"
    fi

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
