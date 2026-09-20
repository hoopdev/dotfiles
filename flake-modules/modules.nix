# Self-exported NixOS modules — hosts pick them up via inputs.self.nixosModules.X.
_:

{
  flake.nixosModules = {
    default = ../modules/nixos;
    headless = ../modules/nixos/headless.nix;
    nix-ld = ../modules/nixos/nix-ld.nix;
    onepassword = ../modules/nixos/onepassword.nix;
    nix-settings = ../modules/nixos/nix-settings.nix;
    hyprland-cache = ../modules/nixos/hyprland-cache.nix;
  };
}
