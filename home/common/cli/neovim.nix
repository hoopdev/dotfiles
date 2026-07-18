{
  pkgs,
  config,
  lib,
  ...
}:
let
  initLuaSource = ./init.lua;
  dotfilesDir = config.dotfiles.paths.repo;

  # Obsidian vault paths - can be overridden per host via
  # programs.neovim.obsidianVaults (custom option declared below).
  obsidianVaults = config.programs.neovim.obsidianVaults;

  # Generate Lua code for obsidian vaults
  vaultsLua = lib.concatMapStringsSep ",\n    " (vault: ''
    {
      name = "${vault.name}",
      path = "${vault.path}",
    }'') obsidianVaults;
in
{
  # Allow setting obsidian vaults from host config. (Custom option grafted
  # onto the stock home-manager programs.neovim namespace; it replaces the
  # old programs.nixvim.obsidianVaults — nixvim itself was removed 2026-07-11:
  # it was only a thin shell around init.lua, and its pandoc-rendered option
  # man pages broke `nh switch` on the pinned nixpkgs.)
  options.programs.neovim.obsidianVaults = lib.mkOption {
    type = lib.types.listOf (lib.types.attrsOf lib.types.str);
    default = [ ];
    description = "List of Obsidian vault configurations";
  };

  config.programs.neovim = {
    enable = true;
    defaultEditor = true;
    # Keep current behavior explicit while home.stateVersion remains 24.05.
    withPython3 = true;
    withRuby = true;

    # Load external Lua configuration (init.lua) with vault paths injected.
    # home-manager writes this as ~/.config/nvim/init.lua.
    initLua = ''
      -- Obsidian vault configuration (injected from Nix)
      vim.g.obsidian_vaults = {
        ${vaultsLua}
      }
    ''
    + builtins.readFile initLuaSource;

    # Python packages for Neovim plugins
    extraPython3Packages =
      ps: with ps; [
        jupyter-client
        jupytext
        pynvim
      ];

    # Add lazy.nvim to the runtimepath for bootstrapping; plugin management
    # itself is done by lazy.nvim from init.lua, exactly as before.
    plugins = with pkgs.vimPlugins; [
      lazy-nvim
    ];
  };

  # Copy init.lua to chezmoi dotfiles directory on activation
  config.home.activation.syncNeovimConfig = lib.mkIf (dotfilesDir != null) (
    config.lib.dag.entryAfter [ "writeBoundary" ] ''
      if [ -d "${dotfilesDir}" ]; then
        $DRY_RUN_CMD mkdir -p "${dotfilesDir}/chezmoi/dot_config/nvim"
        $DRY_RUN_CMD cp -f ${initLuaSource} "${dotfilesDir}/chezmoi/dot_config/nvim/init.lua"
        echo "Synced init.lua to chezmoi dotfiles"
      fi
    ''
  );
}
