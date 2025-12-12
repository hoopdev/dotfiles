{ pkgs, config, ... }:
let
  initLuaSource = ./init.lua;
  dotfilesDir = "${config.home.homeDirectory}/git/dotfiles";
in
{
  programs.nixvim = {
    enable = true;

    # Load external Lua configuration (init.lua)
    extraConfigLua = builtins.readFile initLuaSource;

    # Python packages for Neovim plugins
    extraPython3Packages = ps: with ps; [
      jupyter-client
      jupytext
      pynvim
    ];

    # Add lazy.nvim plugin to be available for bootstrapping
    extraPlugins = with pkgs.vimPlugins; [
      lazy-nvim
    ];

    # Disable nixvim's built-in lazy plugin management
    # We use lazy.nvim directly in init.lua for full control
    plugins = {
      lazy = {
        enable = false;
      };
    };

    # Performance settings
    performance = {
      byteCompileLua = {
        enable = true;
        nvimRuntime = true;
        plugins = true;
      };
    };
  };

  # Copy init.lua to chezmoi dotfiles directory on activation
  home.activation.syncNeovimConfig = config.lib.dag.entryAfter ["writeBoundary"] ''
    if [ -d "${dotfilesDir}" ]; then
      $DRY_RUN_CMD mkdir -p "${dotfilesDir}/dot_config/nvim"
      $DRY_RUN_CMD cp -f ${initLuaSource} "${dotfilesDir}/dot_config/nvim/init.lua"
      echo "Synced init.lua to chezmoi dotfiles"
    fi
  '';
}