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
  };

  config = {
    home.pointerCursor.enable = lib.mkIf (!pkgs.stdenv.hostPlatform.isDarwin) true;

    # Stylix still uses the deprecated Rofi font option. Do not generate
    # unused Rofi settings on hosts where the program is disabled.
    stylix.targets.rofi.enable = lib.mkDefault config.programs.rofi.enable;
  };
}
