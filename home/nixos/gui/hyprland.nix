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
      ];
      monitor = [ "eDP-1, 2160x1440@60, 0x0, 1" ];
      env = [
        "GDK_SCALE,2"
        "XCURSOR_SIZE,32"
        "GTK_THEME,Nord"
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
        resize_on_border = true;
      };
      decoration = {
        rounding = 10;
        blur = {
          enabled = true;
          size = 3;
          passes = 1;
          xray = true;
          ignore_opacity = true;
          new_optimizations = true;
        };
      };
      "$mod" = "ALT";
      "$term" = "wezterm";
      bind =
        [
          "$mod, V, exec, vivaldi"
          "$mod, C, exec, wezterm"
          "$mod, SPACE, exec, wofi --show drun"
          "$mod SHIFT, M, exit"
          ", XF86AudioMute, exec, pamixer -t"
          ", XF86AudioRaiseVolume, exec, pamixer -i 10"
          ", XF86AudioLowerVolume, exec, pamixer -d 10"
          ", XF86MonBrightnessUp, exec, brightnessctl set +10%"
          ", XF86MonBrightnessDown, exec, brightnessctl set 10%-"
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
