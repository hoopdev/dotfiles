{
  pkgs,
  config,
  ...
}:
{
  programs.alacritty = {
    enable = true;
    settings = {
      colors = {
        bright = {
            black = "0x${config.colorScheme.palette.base00}";
            blue = "0x${config.colorScheme.palette.base0D}";
            cyan = "0x${config.colorScheme.palette.base0C}";
            green = "0x${config.colorScheme.palette.base0B}";
            magenta = "0x${config.colorScheme.palette.base0E}";
            red = "0x${config.colorScheme.palette.base08}";
            white = "0x${config.colorScheme.palette.base06}";
            yellow = "0x${config.colorScheme.palette.base09}";
        };
        cursor = {
            text = "0x${config.colorScheme.palette.base06}";
            cursor = "0x${config.colorScheme.palette.base06}";
        };
        normal = {
          black = "0x${config.colorScheme.palette.base00}";
          blue = "0x${config.colorScheme.palette.base0D}";
          cyan = "0x${config.colorScheme.palette.base0C}";
          green = "0x${config.colorScheme.palette.base0B}";
          magenta = "0x${config.colorScheme.palette.base0E}";
          red = "0x${config.colorScheme.palette.base08}";
          white = "0x${config.colorScheme.palette.base06}";
          yellow = "0x${config.colorScheme.palette.base0A}";
        };
        primary = {
          background = "0x${config.colorScheme.palette.base00}";
          foreground = "0x${config.colorScheme.palette.base06}";
        };
      };
      font = rec {
        normal.family = "HackGen Console NF";
        size = 14;
        bold = {
          style = "Bold";
        };
      };
      window.padding = {
        x = 2;
        y = 2;
      };
      window.opacity = 0.9;
      terminal.shell.program = "${pkgs.zsh}/bin/zsh";
      cursor.style = "Beam";

    };
  };
}
