# ThinkPad-specific Hyprland values. The shared desktop profile
# (home/nixos/gui/hyprland.nix) carries everything hardware-independent.
_: {
  wayland.windowManager.hyprland.settings = {
    monitor = [ "eDP-1, 2160x1440@60, 0x0, 1" ];
    # 2160x1440 on 13" → 2x scaling for GTK apps. The matching cursor size is
    # set via stylix.cursor.size in configuration.nix so every toolkit agrees.
    env = [ "GDK_SCALE,2" ];
    # TrackPoint on the keyboard (USB device)
    device = {
      name = "synaptics-tm3203-003";
      sensitivity = 0;
      accel_profile = "flat";
    };
  };
}
