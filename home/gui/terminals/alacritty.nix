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
        primary = {
          background = "0x${config.colorScheme.palette.base03}";
          foreground = "0x${config.colorScheme.palette.base06}";
          dim_foreground = "0x${config.colorScheme.palette.base05}";
          bright_foreground = "0x${config.colorScheme.palette.base06}";
        };
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
        cursor = {
            text = "0x${config.colorScheme.palette.base06}";
            cursor = "0x${config.colorScheme.palette.base06}";
        };
	search = {
	    matches =  {
	        foreground = "0x${config.colorScheme.palette.base06}";
	        background = "0x${config.colorScheme.palette.base02}";
	    };
	    focused_match =  {
	        foreground = "0x${config.colorScheme.palette.base06}";
	        background = "0x${config.colorScheme.palette.base02}";
	    };
	};
        footer_bar = {
            foreground = "0x${config.colorScheme.palette.base06}";
            background = "0x${config.colorScheme.palette.base06}";
        };
	hints = {
	    start =  {
	        foreground = "0x${config.colorScheme.palette.base06}";
	        background = "0x${config.colorScheme.palette.base02}";
	    };
	    end =  {
	        foreground = "0x${config.colorScheme.palette.base06}";
	        background = "0x${config.colorScheme.palette.base02}";
	    };
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
