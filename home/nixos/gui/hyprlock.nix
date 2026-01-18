# Hyprlock - GPU-accelerated lock screen for Hyprland
# Styling is fully managed by Stylix
{ ... }:
{
  programs.hyprlock = {
    enable = true;
    settings = {
      general = {
        hide_cursor = true;
        ignore_empty_input = false;
        immediate_render = false;
        text_trim = true;
        fractional_scaling = 2; # auto
      };
      # Background, input-field, and label are managed by Stylix
    };
  };
}
