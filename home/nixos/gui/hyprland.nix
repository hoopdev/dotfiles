{
  lib,
  pkgs,
  osConfig ? { },
  ...
}:
{
  home.packages = [
    pkgs.hyprpicker
    pkgs.hyprpaper
    pkgs.wl-clipboard
    pkgs.cliphist
    pkgs.grimblast
    pkgs.brightnessctl
    pkgs.playerctl
    pkgs.pamixer
  ];

  # Wallpaper: Stylix's hyprpaper target enables hyprpaper and sets the
  # wallpaper from stylix.image — do not define services.hyprpaper here.

  wayland.windowManager.hyprland = {
    enable = true;
    # Preserve the existing generated Hyprland configuration format across
    # future Home Manager state-version bumps.
    configType = "hyprlang";
    xwayland.enable = true;
    settings = {
      exec-once = [
        # The bar/shell (wayle) is a systemd user service — see wayle.nix.
        "fcitx5 -d --replace"
        "swayosd-server"
        "wl-paste --watch cliphist store"
      ]
      # Keeps the 1Password SSH agent socket alive across logins; without
      # this the socket file persists but nothing listens on it, so
      # ssh-add/git-over-ssh silently fail with "Permission denied".
      # Only on hosts that selected the `onepassword` system profile.
      ++ lib.optional (osConfig.programs._1password-gui.enable or false) "1password --silent";
      # monitor, HiDPI env (GDK_SCALE) and the per-device input block are
      # hardware-specific and live in hosts/<name>/home.nix. GTK theme and
      # cursor (XCURSOR_SIZE) are set by Stylix — do not override them here.
      env = [
        # IME (Fcitx5) support — GTK4 uses native Wayland text-input-v3,
        # so GTK_IM_MODULE is intentionally unset to silence the warning.
        "QT_IM_MODULE,fcitx"
        "XMODIFIERS,@im=fcitx"
        "INPUT_METHOD,fcitx"
        "GLFW_IM_MODULE,ibus"
      ];
      xwayland = {
        force_zero_scaling = true;
      };
      misc = {
        disable_hyprland_logo = true;
      };
      input = {
        # Enable all pointer devices
        sensitivity = 0;
        accel_profile = "flat";
        touchpad = {
          natural_scroll = true;
          tap-to-click = true;
          clickfinger_behavior = true;
        };
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
      bind = [
        "$mod, V, exec, vivaldi"
        "$mod, C, exec, wezterm"
        "$mod, L, exec, hyprlock"
        "$mod, SPACE, exec, wofi --show drun"
        "$mod, Escape, exec, wlogout"
        "$mod SHIFT, V, exec, cliphist list | wofi -d | cliphist decode | wl-copy"
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
