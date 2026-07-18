{
  lib,
  pkgs,
  ...
}:

{
  imports = [
    ./profiles.nix
    ./cli
  ];

  home = {
    sessionVariables = {
      NIXPKGS_ALLOW_UNFREE = "1";
    }
    // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
      NPM_CONFIG_PREFIX = "$HOME/.npm-global";
    };

    sessionPath = lib.optionals (!pkgs.stdenv.isDarwin) [
      "$HOME/.npm-global/bin"
    ];

  };

  programs.home-manager.enable = true;

  # The home-manager stylix submodule checks its release (26.05) against
  # home-manager's (26.11). Both track nixpkgs-unstable so they're compatible —
  # skip the check. Set here (not just lib/stylix.nix) because for integrated
  # NixOS/darwin setups lib/stylix.nix is imported at the system level, leaving
  # this HM-level instance — the one that emits the warning — unconfigured.
  stylix.enableReleaseChecks = false;
}
