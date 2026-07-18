# Hyprland's binary cache is useful only on hosts that run Hyprland. Keeping it
# out of the base profile avoids broadening trust on servers and WSL.
_: {
  nix.settings = {
    substituters = [ "https://hyprland.cachix.org" ];
    trusted-public-keys = [
      "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
    ];
  };
}
