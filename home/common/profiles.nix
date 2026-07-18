{
  lib,
  pkgs,
  config,
  ...
}:
{
  options.dotfiles.paths = {
    repo = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = "${config.home.homeDirectory}/dotfiles";
      description = "Location of this dotfiles checkout on the current host.";
    };
    devSource = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Optional local checkout of hoopdev/dev used by development wrappers.";
    };
  };

  config = lib.mkIf (!pkgs.stdenv.isDarwin) {
    home.pointerCursor.enable = true;
  };
}
