{ lib, pkgs, ... }:

{
  imports = [
    ../common
    ./cli
    ./gui
  ];

  xdg = {
    enable = true;
    userDirs = {
      enable = true;
      createDirectories = true;
      extraConfig = {
        desktop = "$HOME/Desktop";
        download = "$HOME/Downloads";
        documents = "$HOME/Documents";
        music = "$HOME/Music";
        videos = "$HOME/Videos";
      };
    };
  };
}
