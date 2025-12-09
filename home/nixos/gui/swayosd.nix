{ pkgs, ... }:

{
  home.packages = with pkgs; [
    swayosd
  ];

  services.swayosd = {
    enable = true;
    topMargin = 0.5;
  };
}
