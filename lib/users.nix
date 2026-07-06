# User-account helper for NixOS hosts. `mkUser` returns a module (so it receives
# its own `pkgs`); import the result and pass only the host-specific groups:
#
#   imports = [
#     ((import ../../lib/users.nix).mkUser {
#       username = "ktaga";
#       extraGroups = [ "networkmanager" "audio" "video" "docker" ];
#     })
#   ];
#
# The "wheel" group and `programs.zsh.enable` are always included.
{
  mkUser =
    {
      username,
      extraGroups ? [ ],
      description ? username,
    }:
    { pkgs, ... }:
    {
      users.users.${username} = {
        isNormalUser = true;
        inherit description;
        shell = pkgs.zsh;
        extraGroups = [ "wheel" ] ++ extraGroups;
      };
      programs.zsh.enable = true;
    };
}
