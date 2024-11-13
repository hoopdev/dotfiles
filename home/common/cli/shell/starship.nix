{
  config,
  ...
}:
{
  programs.starship = {
    enable = true;
    enableNushellIntegration = true;
    enableZshIntegration = true;

    # settings = builtins.fromTOML (builtins.readFile ./starship.toml);
    settings = {

      format = "[░▒▓](#${config.colorScheme.palette.base06})[  ](bg:#${config.colorScheme.palette.base06} fg:#${config.colorScheme.palette.base00})[](bg:#${config.colorScheme.palette.base0A} fg:#${config.colorScheme.palette.base06})$directory[](fg:#${config.colorScheme.palette.base0A} bg:#${config.colorScheme.palette.base0B})$git_branch$git_status[](fg:#${config.colorScheme.palette.base0B} bg:#${config.colorScheme.palette.base0C})$time[ ](fg:#${config.colorScheme.palette.base0C})$character";

      directory = {
        format = "[ $path ]($style)";
        style = "fg:#${config.colorScheme.palette.base01} bg:#${config.colorScheme.palette.base0A}";
        truncation_length = 3;
        truncation_symbol = "…/";
        substitutions = {
          Documents = "󰈙 ";
          Downloads = " ";
          Music = " ";
          Pictures = " ";
        };
      };

      git_branch = {
        format = "[[ $symbol $branch ](fg:#${config.colorScheme.palette.base01} bg:#${config.colorScheme.palette.base0B})]($style)";
        style = "bg:#${config.colorScheme.palette.base04}";
        symbol = "";
      };

      git_status = {
        format = "[[($all_status$ahead_behind )](fg:#${config.colorScheme.palette.base01} bg:#${config.colorScheme.palette.base0B})]($style)";
        style = "bg:#${config.colorScheme.palette.base04}";
      };

      time = {
        disabled = false;
        format = "[[  $time ](fg:#${config.colorScheme.palette.base01} bg:#${config.colorScheme.palette.base0C})]($style)";
        style = "bg:#${config.colorScheme.palette.base06}";
        time_format = "%R";
      };

    };
  };
}
