{
  pkgs,
  config,
  ...
}:
let
  isLinux = pkgs.stdenv.isLinux;
  isDarwin = pkgs.stdenv.isDarwin;
in
{
  programs.wezterm = {
    enable = true;
    extraConfig = ''
      local wezterm = require("wezterm")

      return {
        color_scheme = "nord",
        window_background_opacity = 0.9,

        font = wezterm.font("HackGen Console NF", { weight = "Regular", stretch = "Normal", style = "Normal" }),
        font_size = 14.0,

        window_padding = {
          left = 10,
          right = 10,
          top = 10,
          bottom = 10,
        },

        use_fancy_tab_bar = false,
        hide_tab_bar_if_only_one_tab = true,
        window_decorations = "NONE",

        front_end = "WebGpu",
        enable_wayland = ${if isLinux then "true" else "false"},
        use_ime = true,
        check_for_updates = false,
      }
    '';
  };
}
