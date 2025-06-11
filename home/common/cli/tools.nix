{ pkgs, ... }:
{
  home.packages = with pkgs; [
    bat
    eza
    fd
    fx
    fzf
    difftastic
    dust
    procs
    bottom
    ghq
    httpie
    imagemagick
    jq
    zoxide
    unar
    unrar
    unzip
    zip
    zellij
    gotop
    yazi
    ripgrep
    hackgen-nf-font
    nixfmt-rfc-style
    quarto
    fastfetch
    _1password-cli
    claude-code
  ];
  programs.zoxide = {
    enable = true;
    package = pkgs.zoxide;
    enableNushellIntegration = true;
    enableZshIntegration = true;
  };
  programs.zellij = {
    enable = true;
    package = pkgs.zellij;
    enableZshIntegration = false;
    settings = {
      theme = "nord";
    };
  };
}
