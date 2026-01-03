{ config, lib, ... }:
let
  # Stylix colors (base16 palette)
  colors = config.lib.stylix.colors;
in
{
  programs.starship = {
    enable = true;
    enableNushellIntegration = true;
    enableZshIntegration = true;

    settings = lib.mkForce {
      # Ubuntu-specific format with Ubuntu logo
      format = "[░▒▓](#${colors.base06})[ 󰕈 ](bg:#${colors.base06} fg:#${colors.base00})[](bg:#${colors.base0A} fg:#${colors.base06})$directory[](fg:#${colors.base0A} bg:#${colors.base0B})$git_branch$git_status[](fg:#${colors.base0B} bg:#${colors.base0C})$time[ ](fg:#${colors.base0C})$character";

      directory = {
        format = "[ $path ]($style)";
        style = "fg:#${colors.base01} bg:#${colors.base0A}";
        truncation_length = 3;
        truncation_symbol = "…/";
        substitutions = {
          Documents = "󰈙 ";
          Downloads = " ";
          Music = " ";
          Pictures = " ";
        };
      };

      git_branch = {
        format = "[[ $symbol $branch ](fg:#${colors.base01} bg:#${colors.base0B})]($style)";
        style = "bg:#${colors.base04}";
        symbol = "";
      };

      git_status = {
        format = "[[($all_status$ahead_behind )](fg:#${colors.base01} bg:#${colors.base0B})]($style)";
        style = "bg:#${colors.base04}";
      };

      time = {
        disabled = false;
        format = "[[  $time ](fg:#${colors.base01} bg:#${colors.base0C})]($style)";
        style = "bg:#${colors.base06}";
        time_format = "%R";
      };
    };
  };
}
