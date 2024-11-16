{ pkgs, ... }:
{
  home.packages = with pkgs; [
    wofi
    wofi-emoji
    hyprpicker
    hypridle
    hyprpaper
    hyprlock
    wl-clipboard
    brightnessctl
    playerctl
  ];

  programs.waybar = {
    enable = true;
    systemd.enable = true;
    settings = {
      mainBar = {
        layer = "top";
        position = "top";
        height = 30;
        modules-left = [
          "hyprland/workspaces"
          "cpu"
          "memory"
          "temperature"
          "disk"
        ];
        modules-center = [ "hyprland/window" ];
        modules-right = [
          "idle_inhibitor"
          "pulseaudio"
          "backlight"
          "battery"
          "clock"
          "keyboard-state"
          "tray"
        ];
      };
    };
  };

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
      bind =
        [
          "$mod, V, exec, vivaldi"
          "$mod, C, exec, wezterm"
          "$mod SHIFT, M, exit"
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
