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
}