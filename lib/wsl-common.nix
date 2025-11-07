{ lib, pkgs, inputs, username ? "ktaga", enableDockerGroup ? false, ... }:

{
  # Import NixOS-WSL modules
  imports = [
    inputs.nixos-wsl.nixosModules.wsl
  ];

  # WSL configuration
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

  # Enable Docker
  virtualisation.docker = {
    enable = true;
  };

  # Fonts for WSL
  fonts.packages = with pkgs; [
    noto-fonts-cjk-sans
    noto-fonts-color-emoji
  ];

  # Shell configuration
  programs.zsh.enable = true;
  environment.pathsToLink = [ "/share/zsh" ];
  environment.shells = [ pkgs.zsh ];

  # User configuration
  users.users.${username} = {
    isNormalUser = true;
    shell = pkgs.zsh;
    extraGroups = [
      "wheel"
    ] ++ lib.optionals enableDockerGroup [ "docker" ];
  };

  # State version
  system.stateVersion = "24.11";
}
