# Shared NixOS-WSL settings. Called as a function from the host:
#
#   imports = [ (import ../../lib/wsl-common.nix { inherit lib pkgs inputs; username = primaryUser; }) ];
#
# The user account itself comes from lib/users.nix like every other NixOS
# host; `system.stateVersion` stays in the host file.
{
  lib,
  pkgs,
  inputs,
  username ? "ktaga",
  enableDockerGroup ? false,
  ...
}:

{
  imports = [
    inputs.nixos-wsl.nixosModules.wsl
    ((import ./users.nix).mkUser {
      inherit username;
      extraGroups = lib.optionals enableDockerGroup [ "docker" ];
    })
  ];

  wsl = {
    enable = true;
    defaultUser = username;
    wslConf = {
      interop.appendWindowsPath = false;
      automount = {
        root = "/mnt";
        enabled = true;
      };
    };
  };

  virtualisation.docker.enable = true;

  # Fonts for GUI apps forwarded through WSLg.
  fonts.packages = with pkgs; [
    noto-fonts-cjk-sans
    noto-fonts-color-emoji
  ];

  environment.pathsToLink = [ "/share/zsh" ];
  environment.shells = [ pkgs.zsh ];
}
