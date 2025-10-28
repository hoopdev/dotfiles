{
  pkgs,
  config,
  ...
}:
let
  weztermConfigSource = ./wezterm.lua;
  dotfilesDir = "${config.home.homeDirectory}/git/dotfiles";
in
{
  programs.wezterm = {
    enable = true;
    extraConfig = builtins.readFile weztermConfigSource;
  };

  # Copy wezterm.lua to chezmoi dotfiles directory on activation
  home.activation.syncWeztermConfig = config.lib.dag.entryAfter ["writeBoundary"] ''
    if [ -d "${dotfilesDir}" ]; then
      $DRY_RUN_CMD mkdir -p "${dotfilesDir}/dot_config/wezterm"
      $DRY_RUN_CMD cp -f ${weztermConfigSource} "${dotfilesDir}/dot_config/wezterm/wezterm.lua"
      echo "Synced wezterm.lua to chezmoi dotfiles"
    fi
  '';
}
