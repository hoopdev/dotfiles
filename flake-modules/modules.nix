# Self-exported NixOS modules — hosts pick them up via inputs.self.nixosModules.X.
_:

{
  flake.nixosModules = {
    default = ../modules/nixos;
    nix-ld = ../modules/nixos/nix-ld.nix;
    onepassword = ../modules/nixos/onepassword.nix;
    nix-settings = ../modules/nixos/nix-settings.nix;
  };
}
