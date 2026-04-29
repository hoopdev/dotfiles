# Aggregate of NixOS modules previously bundled in lib/nixos-common.nix.
# Hosts can either import this kitchen-sink module or pick individual ones:
#
#   imports = [ inputs.self.nixosModules.default ];        # all
#   imports = [ inputs.self.nixosModules.nix-ld            # subset
#               inputs.self.nixosModules.nix-settings ];
{ ... }:

{
  imports = [
    ./nix-ld.nix
    ./onepassword.nix
    ./nix-settings.nix
  ];
}
