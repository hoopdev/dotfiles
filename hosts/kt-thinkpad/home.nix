{
  lib,
  pkgs,
  username,
  inputs,
  ...
}:

{
  imports = [
    ../../home/nixos
  ];

  home = {
    inherit username;
    homeDirectory = "/home/${username}";
    stateVersion = "24.05";
  };

  home.packages = [
    inputs.hyprpanel.packages.x86_64-linux.default
  ];
}
