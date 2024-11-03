{
  inputs,
  pkgs,
  ...
}:
{
  programs.wezterm = {
    package = inputs.wezterm.packages.${pkgs.system}.default;
    enable = true;
    extraConfig = builtins.readFile ./wezterm.lua;
  };

    # ranger config - enable image preview on wezterm
    ".config/ranger/rc.conf".text = ''
      set preview_images true
      set preview_images_method iterm2
    '';
  };
}