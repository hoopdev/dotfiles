{ inputs, pkgs, ... }:
{
  home.packages = with pkgs; [
    hyprpicker
    hypridle
    hyprpaper
    hyprlock
    wl-clipboard
    brightnessctl
    playerctl
    pamixer
  ];

  services.dunst.enable = true;

  services.hyprpaper = {
    enable = true;
    settings = {
      preload = [ "~/dotfiles/wallpaper/wallpaper_enoshima.jpg" ];
      wallpaper = [
        "eDP-1,~/dotfiles/wallpaper/wallpaper_enoshima.jpg"
      ];
    };
  };

  wayland.windowManager.hyprland = {
    enable = true;
    xwayland.enable = true;
    settings = {
      exec-once = [
        "hyprpanel"
        "fcitx5 -d --replace"
        "swayosd-server"
      ];
      monitor = [ "eDP-1, 2160x1440@60, 0x0, 1" ];
      env = [
        "GDK_SCALE,2"
        "XCURSOR_SIZE,32"
        "GTK_THEME,Nordic"
      ];
      xwayland = {
        force_zero_scaling = true;
      };
      misc = {
        disable_hyprland_logo = true;
      };
      general = {
        gaps_in = 5;
        gaps_out = 5;
        border_size = 2;
        # Border colors managed by Stylix
        # "col.active_border" = "rgba(88c0d0ff) rgba(81a1c1ff) 45deg";
        # "col.inactive_border" = "rgba(4c566aaa)";
        resize_on_border = true;
      };
      decoration = {
        rounding = 12;
        active_opacity = 1.0;
        inactive_opacity = 0.92;
        blur = {
          enabled = true;
          size = 8;
          passes = 3;
          xray = true;
          ignore_opacity = true;
          new_optimizations = true;
          noise = 0.02;
          contrast = 1.0;
          brightness = 1.0;
        };
        shadow = {
          enabled = true;
          range = 20;
          render_power = 3;
          # Shadow colors managed by Stylix
          # color = "rgba(1a1a1aee)";
          # color_inactive = "rgba(1a1a1a99)";
        };
      };
      animations = {
        enabled = true;
        bezier = [
          "easeOutQuint, 0.23, 1, 0.32, 1"
          "easeInOutCubic, 0.65, 0, 0.35, 1"
          "linear, 0, 0, 1, 1"
          "almostLinear, 0.5, 0.5, 0.75, 1.0"
          "quick, 0.15, 0, 0.1, 1"
        ];
        animation = [
          "global, 1, 10, default"
          "border, 1, 5.39, easeOutQuint"
          "windows, 1, 4.79, easeOutQuint"
          "windowsIn, 1, 4.1, easeOutQuint, popin 87%"
          "windowsOut, 1, 1.49, linear, popin 87%"
          "fadeIn, 1, 1.73, almostLinear"
          "fadeOut, 1, 1.46, almostLinear"
          "fade, 1, 3.03, quick"
          "layers, 1, 3.81, easeOutQuint"
          "layersIn, 1, 4, easeOutQuint, fade"
          "layersOut, 1, 1.5, linear, fade"
          "fadeLayersIn, 1, 1.79, almostLinear"
          "fadeLayersOut, 1, 1.39, almostLinear"
          "workspaces, 1, 1.94, almostLinear, fade"
          "workspacesIn, 1, 1.21, almostLinear, fade"
          "workspacesOut, 1, 1.94, almostLinear, fade"
        ];
      };
      "$mod" = "ALT";
      "$term" = "wezterm";
      bind =
        [
          "$mod, V, exec, vivaldi"
          "$mod, C, exec, wezterm"
          "$mod, SPACE, exec, wofi --show drun"
          "$mod SHIFT, M, exit"
          ", XF86AudioMute, exec, swayosd-client --output-volume mute-toggle"
          ", XF86AudioRaiseVolume, exec, swayosd-client --output-volume raise"
          ", XF86AudioLowerVolume, exec, swayosd-client --output-volume lower"
          ", XF86MonBrightnessUp, exec, swayosd-client --brightness raise"
          ", XF86MonBrightnessDown, exec, swayosd-client --brightness lower"
          ", Print, exec, grimblast copy area"
        ]
        ++ (
          # workspaces
          # binds $mod + [shift +] {1..9} to [move to] workspace {1..9}
          builtins.concatLists (
            builtins.genList (
              i:
              let
                ws = i + 1;
              in
              [
                "$mod, code:1${toString i}, workspace, ${toString ws}"
                "$mod SHIFT, code:1${toString i}, movetoworkspace, ${toString ws}"
              ]
            ) 9
          )
        );
    };
  };
}
