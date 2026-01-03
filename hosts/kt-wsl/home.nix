{ lib, pkgs, username, inputs, ... }:

{
  imports = [
    ../../home/common/cli
    inputs.nixvim.homeModules.nixvim
  ];

  home = {
    inherit username;
    homeDirectory = "/home/${username}";
    stateVersion = "24.05";
  };
}