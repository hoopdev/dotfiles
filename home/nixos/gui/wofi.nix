{ ... }:
{
  programs.wofi = {
    enable = true;
    settings = {
      term = "wezterm";
      insensitive = true;
      normal_window = true;
      gtk-dark = true;
      prompt = "Search...";
      width = "50%";
      height = "40%";
      key_up = "Ctrl-k";
      key_down = "Ctrl-j";
    };
  };
}
