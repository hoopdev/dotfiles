# Nix daemon settings shared across all NixOS hosts: store optimisation and
# experimental features. Hyprland comes from nixpkgs (cache.nixos.org), so no
# extra substituter is needed.
_:

{
  nix.settings = {
    auto-optimise-store = true;
    experimental-features = [
      "nix-command"
      "flakes"
    ];
  };
}
