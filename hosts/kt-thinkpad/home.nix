{ inputs, pkgs, ... }:
{
  home.packages = [
    inputs.hyprpanel.packages.${pkgs.stdenv.hostPlatform.system}.default
  ];
}
