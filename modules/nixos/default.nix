# Minimal NixOS baseline. Optional services (such as 1Password GUI and
# Hyprland's cache) are selected through a host's `systemProfiles` metadata.
#
#   imports = [ inputs.self.nixosModules.default ];
{ ... }:

{
  imports = [
    ./nix-ld.nix
    ./nix-settings.nix
  ];
}
