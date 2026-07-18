{ config, lib, ... }:
{
  programs.nh = {
    enable = true;
    clean = {
      enable = true;
      dates = "weekly";
      extraArgs = "--keep-since 7d --keep 5";
    };
  }
  // lib.optionalAttrs (config.dotfiles.paths.repo != null) {
    # The checkout location is host-configurable rather than tied to one layout.
    flake = config.dotfiles.paths.repo;
  };
}
