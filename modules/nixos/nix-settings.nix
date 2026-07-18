# Nix daemon settings shared across all NixOS hosts: store optimisation and
# experimental features. Hyprland's cache lives in hyprland-cache.nix and is
# selected only by desktop hosts.
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
