
{ pkgs, ... }:
{
  programs.zsh = {
    enable = true;
    configDir = ".config/nushell";
  };
}
