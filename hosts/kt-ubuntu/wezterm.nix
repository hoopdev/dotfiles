{ pkgs, config, lib, ... }:
{
  # Override WezTerm font configuration for kt-ubuntu
  programs.wezterm = {
    enable = true;
    extraConfig = lib.mkForce ''
      local wezterm = require("wezterm")

      return {
        color_scheme = "nord",
        window_background_opacity = 0.9,

        -- Use Nerd Font for proper icon display
        font = wezterm.font("JetBrainsMono Nerd Font Mono", { weight = "Regular", stretch = "Normal", style = "Normal" }),
        font_size = 13.0,

        window_padding = {
          left = 10,
          right = 10,
          top = 10,
          bottom = 10,
        },

        use_fancy_tab_bar = false,
        hide_tab_bar_if_only_one_tab = true,
        window_decorations = "RESIZE",

        front_end = "WebGpu",
        enable_wayland = false,  -- code-server environment
        use_ime = true,
        check_for_updates = false,
      }
    '';
  };
}
