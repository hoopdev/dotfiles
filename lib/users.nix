{ lib, pkgs, ... }:

{
  # Helper function to create user configuration
  mkUser = { username, extraGroups ? [], description ? username, shell ? pkgs.zsh }: {
    users.users.${username} = {
      isNormalUser = true;
      inherit description shell;
      extraGroups = [ "wheel" ] ++ extraGroups;
    };
    programs.zsh.enable = true;
  };
}
