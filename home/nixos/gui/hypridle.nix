# Hypridle - Idle daemon for Hyprland
{ ... }:
{
  services.hypridle = {
    enable = true;
    settings = {
      general = {
        lock_cmd = "pidof hyprlock || hyprlock";
        before_sleep_cmd = "loginctl lock-session";
        after_sleep_cmd = "hyprctl dispatch dpms on";
      };

      listener = [
        # Lock screen after 5 minutes (300 seconds)
        {
          timeout = 300;
          on-timeout = "loginctl lock-session";
        }
        # Turn off display after 5.5 minutes (330 seconds)
        {
          timeout = 330;
          on-timeout = "hyprctl dispatch dpms off";
          on-resume = "hyprctl dispatch dpms on";
        }
        # Suspend after 30 minutes (1800 seconds)
        {
          timeout = 1800;
          on-timeout = "systemctl suspend";
        }
      ];
    };
  };
}
