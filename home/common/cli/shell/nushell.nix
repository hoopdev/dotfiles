{ pkgs, ... }:
{
  programs.nushell = {
    enable = true;
    shellAliases = import ./aliases.nix;
    extraConfig = ''
      $env.config = {
       show_banner: false,
       completions: {
         case_sensitive: false # case-sensitive completions
         quick: true    # set to false to prevent auto-selecting completions
         partial: true    # set to false to prevent partial filling of the prompt
         algorithm: "fuzzy"    # prefix or fuzzy
       }
      } 
      $env.PATH = ($env.PATH | 
      split row (char esep) |
      prepend /home/myuser/.apps |
      append /usr/bin/env
      )
    '';
  };
}
