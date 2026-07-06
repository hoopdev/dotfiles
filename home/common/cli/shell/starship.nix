{
  pkgs,
  config,
  lib,
  ...
}:
let
  # Stylix colors (base16 palette)
  inherit (config.lib.stylix) colors;

  systemlogo =
    if pkgs.stdenv.hostPlatform.system == "x86_64-linux" then
      ""
    else if pkgs.stdenv.hostPlatform.system == "aarch64-linux" then
      ""
    else if pkgs.stdenv.hostPlatform.system == "x86_64-darwin" then
      ""
    else if pkgs.stdenv.hostPlatform.system == "aarch64-darwin" then
      ""
    else
      "";
in
{
  options.programs.starship.systemLogo = lib.mkOption {
    type = lib.types.str;
    default = systemlogo;
    description = "Glyph shown in the starship prompt's leading system segment; hosts can override it (e.g. an Ubuntu logo).";
  };

  config.programs.starship = {
    enable = true;
    enableNushellIntegration = true;
    enableZshIntegration = true;

    settings = {

      format = "[░▒▓](#${colors.base06})[ ${config.programs.starship.systemLogo} ](bg:#${colors.base06} fg:#${colors.base00})[](bg:#${colors.base0A} fg:#${colors.base06})$directory[](fg:#${colors.base0A} bg:#${colors.base0B})$git_branch$git_status[](fg:#${colors.base0B} bg:#${colors.base0C})$time[ ](fg:#${colors.base0C})$character";
      #format = "[░▒▓](#${colors.base06})[  ](bg:#${colors.base06} fg:#${colors.base00})[](bg:#${colors.base0A} fg:#${colors.base06})$directory[](fg:#${colors.base0A} bg:#${colors.base0B})$git_branch$git_status[](fg:#${colors.base0B} bg:#${colors.base0C})$time[ ](fg:#${colors.base0C})$character";

      directory = {
        format = "[ $path ]($style)";
        style = "fg:#${colors.base01} bg:#${colors.base0A}";
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
        format = "[[ $symbol $branch ](fg:#${colors.base01} bg:#${colors.base0B})]($style)";
        style = "bg:#${colors.base04}";
        symbol = "";
      };

      git_status = {
        format = "[[($all_status$ahead_behind )](fg:#${colors.base01} bg:#${colors.base0B})]($style)";
        style = "bg:#${colors.base04}";
      };

      time = {
        disabled = false;
        format = "[[  $time ](fg:#${colors.base01} bg:#${colors.base0C})]($style)";
        style = "bg:#${colors.base06}";
        time_format = "%R";
      };

    };
  };
}
