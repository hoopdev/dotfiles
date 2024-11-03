{
#   inputs,
  pkgs,
  ...
}:
{
  programs.wezterm = {
    enable = true;
    extraConfig = builtins.readFile ./wezterm.lua;
  };
  home.file = {
    ".config/ranger/rc.conf".text = ''
      set preview_images true
      set preview_images_method iterm2
    '';
  };
}