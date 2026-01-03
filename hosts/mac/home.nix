{ lib, pkgs, username, inputs, ... }:

{
  imports = [
    ../../home/mac
    # nix-colors disabled - Stylix is imported via home/common
  ];

  home = {
    inherit username;
    homeDirectory = "/Users/${username}";
  };

  home.packages = [
    # Temporarily disabled due to wayland dependency issues on macOS
    # inputs.nixvim.packages.aarch64-darwin.default
  ];
}
