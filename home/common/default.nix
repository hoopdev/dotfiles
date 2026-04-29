{
  lib,
  pkgs,
  username,
  inputs,
  ...
}:

{
  imports = [
    inputs.nixvim.homeModules.nixvim
    ./cli
    ./gui
  ];

  home = {
    inherit username;
    stateVersion = "24.05";

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
}
