{ pkgs, lib, username, inputs, ... }:
{
  imports = [
    ../../home/nixos
    inputs.nix-colors.homeManagerModules.default
  ];

  home = {
    homeDirectory = "/home/${username}";
  };

  home.packages = [
    inputs.nixvim.packages.x86_64-linux.default
    inputs.hyprpanel.packages.x86_64-linux.default
  ];
}
