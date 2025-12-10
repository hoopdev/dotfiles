{ lib, pkgs, config, ... }:

{
  # 1Password
  programs._1password.enable = true;
  programs._1password-gui = {
    enable = true;
    polkitPolicyOwners = [ "ktaga" ];
  };

  # Common Nix settings for all NixOS hosts
  nix = {
    settings = {
      auto-optimise-store = true;
      experimental-features = [
        "nix-command"
        "flakes"
      ];
      # Hyprland cache
      substituters = [ "https://hyprland.cachix.org" ];
      trusted-public-keys = [
        "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
      ];
    };
  };

  # Allow unfree packages
  nixpkgs.config.allowUnfree = true;
}
