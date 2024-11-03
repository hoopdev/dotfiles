{
  pkgs,
  ...
}:
{
  programs.alacritty = {
    enable = true;
    settings = {
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
      window.opacity = 0.90;
      terminal.shell.program = "${pkgs.zsh}/bin/zsh";
      cursor.style = "Beam";

    };
  };
}
