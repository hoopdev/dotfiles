{
  config,
  lib,
  osConfig ? null,
  ...
}:
{
  programs.nh = {
    enable = true;
    # Weekly cleanup only on standalone Home Manager hosts. Under NixOS /
    # nix-darwin the system already runs GC once a week (see
    # flake-modules/shared.nix) and Home Manager generations belong to the
    # system generation, so a second user-level run would be redundant.
    clean = {
      enable = osConfig == null;
      dates = "weekly";
      extraArgs = "--keep-since 7d --keep 5";
    };
  }
  // lib.optionalAttrs (config.dotfiles.paths.repo != null) {
    # The checkout location is host-configurable rather than tied to one layout.
    flake = config.dotfiles.paths.repo;
  };
}
