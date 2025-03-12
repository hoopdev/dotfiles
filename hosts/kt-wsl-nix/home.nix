{ lib, pkgs, username, inputs, ... }:

{
  imports = [
    ../../home/common/cli
    inputs.nix-colors.homeManagerModules.default
  ];

  home = {
    inherit username;
    homeDirectory = "/home/${username}";
  };

  home.packages = [
    inputs.nixvim.packages.x86_64-linux.default
  ];
}
