{ pkgs, ... }:
{
  home.packages = with pkgs; [
    bat
    eza
    fd
    fx
    fzf
    ghq
    httpie
    imagemagick
    jq
    zoxide
    unar
    unrar
    unzip
    zip
	nixfmt-rfc-style
  ];
  programs.zoxide = {
    enable = true;
    package = pkgs.zoxide;
    enableNushellIntegration = true;
    enableZshIntegration = true;
  };
}