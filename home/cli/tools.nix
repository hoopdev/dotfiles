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
    enableZshIntegration = true;
    settings = {
      theme = "nord";
    };
  };
}
