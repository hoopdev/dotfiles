{ pkgs, config, lib, ... }:
let
  initLuaSource = ./init.lua;
  dotfilesDir = "${config.home.homeDirectory}/git/dotfiles";

  # Obsidian vault paths - can be overridden per host
  obsidianVaults = config.programs.nixvim.obsidianVaults or [
    {
      name = "Main";
      path = "${config.home.homeDirectory}/Obsidian/Main";
    }
  ];

  # Generate Lua code for obsidian vaults
  vaultsLua = lib.concatMapStringsSep ",\n    " (vault: ''
    {
      name = "${vault.name}",
      path = "${vault.path}",
    }'') obsidianVaults;
in
{
  # Allow setting obsidian vaults from host config
  options.programs.nixvim.obsidianVaults = lib.mkOption {
    type = lib.types.listOf (lib.types.attrsOf lib.types.str);
    default = [
      {
        name = "Main";
        path = "${config.home.homeDirectory}/Obsidian/Main";
      }
    ];
    description = "List of Obsidian vault configurations";
  };

  config.programs.nixvim = {
    enable = true;

    # Load external Lua configuration (init.lua) with vault paths injected
    extraConfigLua = ''
      -- Obsidian vault configuration (injected from Nix)
      vim.g.obsidian_vaults = {
        ${vaultsLua}
      }
    '' + builtins.readFile initLuaSource;

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
  config.home.activation.syncNeovimConfig = config.lib.dag.entryAfter ["writeBoundary"] ''
    if [ -d "${dotfilesDir}" ]; then
      $DRY_RUN_CMD mkdir -p "${dotfilesDir}/dot_config/nvim"
      $DRY_RUN_CMD cp -f ${initLuaSource} "${dotfilesDir}/dot_config/nvim/init.lua"
      echo "Synced init.lua to chezmoi dotfiles"
    fi
  '';
}