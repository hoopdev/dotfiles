# Wayle — the desktop shell (bar, notification center, dashboard) for Hyprland.
#
# Successor to HyprPanel, which upstream archived and nixpkgs now refuses to
# build ("consider using 'wayle' instead"). Wayle comes from nixpkgs, so no
# extra flake input or second nixpkgs pin is needed.
#
# The home-manager module starts it as a systemd user service bound to
# `wayland.systemd.target` (graphical-session.target, started by Hyprland's
# home-manager systemd integration), so it is NOT listed
# in Hyprland's exec-once. Restart after config edits: `systemctl --user restart wayle`.
#
# Full option reference: https://github.com/wayle-rs/wayle/tree/main/docs/config
{ lib, ... }:
{
  services.wayle = {
    enable = true;
    settings = {
      # Fonts (general.font-*), the Shonan palette (styling.palette) and the bar
      # background opacity come from Stylix's wayle target — do not set them here.
      styling = {
        # "wayle" = use the static palette Stylix injects (no matugen/wallust
        # colour extraction), so the shell follows the Shonan scheme.
        theme-provider = "wayle";
        rounding = "md";
      };

      bar = {
        location = "top";
        rounding = "none";
        layout = [
          {
            monitor = "*";
            left = [
              "dashboard"
              "hyprland-workspaces"
              "window-title"
            ];
            center = [ "clock" ];
            right = [
              {
                name = "status";
                modules = [
                  "network"
                  "bluetooth"
                  "volume"
                  "battery"
                ];
              }
              "notifications"
              "systray"
              "power"
            ];
          }
        ];
      };

      modules = {
        clock.format = "%m/%d (%a) %H:%M";
        hyprland-workspaces = {
          # Always show the 9 workspaces bound to $mod+1..9 in hyprland.nix.
          min-workspace-count = 9;
          show-special = false;
        };
        window-title.label-max-length = 40;
      };

      # Volume/brightness OSD stays with swayosd (bound in hyprland.nix);
      # a second overlay would double every key press.
      osd.enabled = false;

      # Wallpaper is Stylix's job (hyprpaper target), not Wayle's engine.
      wallpaper.engine-enabled = false;
    };
  };

  # Wayle ships its own notification daemon (popups + history dropdown). Two
  # daemons cannot both own org.freedesktop.Notifications, so dunst is off.
  services.dunst.enable = lib.mkForce false;
}
