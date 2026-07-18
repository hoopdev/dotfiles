{ pkgs, ... }:
{
  # macOS uses the Homebrew service declared in the shared Darwin system
  # configuration. Linux hosts opt in through the `syncthing` home profile.
  home.packages = [ pkgs.syncthing ];
  services.syncthing.enable = true;
}
