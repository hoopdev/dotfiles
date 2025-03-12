{ config, pkgs, ... }:
{
  imports = [
    ../common
    ./cli
    ./gui
  ];

  xdg = {
    enable = true;
    userDirs = {
      extraConfig = {
        desktop = "/home/ktaga/Desktop";
        download = "/home/ktaga/Downloads";
        documents = "/home/ktaga/Documents";
        music = "/home/ktaga/Music";
        videos = "/home/ktaga/Videos";
      };
    };
  };
}
