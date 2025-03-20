{ lib, pkgs, username, inputs, ... }:

{
  imports = [
    ../../home/nixos
    inputs.nix-colors.homeManagerModules.default
  ];

  home = {
    inherit username;
    homeDirectory = "/home/${username}";
    stateVersion = "24.05";
  };

  home.packages = [
    inputs.nixvim.packages.x86_64-linux.default
    inputs.hyprpanel.packages.x86_64-linux.default
  ];
}
