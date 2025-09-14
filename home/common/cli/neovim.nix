{ pkgs, inputs, ... }:
let
  # Build dpp and related plugins from flake inputs
  dppPlugins = {
    dpp-vim = pkgs.vimUtils.buildVimPlugin {
      name = "dpp.vim";
      src = inputs.dpp-vim;
    };
    denops-vim = pkgs.vimUtils.buildVimPlugin {
      name = "denops.vim";
      src = inputs.denops-vim;
    };
    dpp-ext-installer = pkgs.vimUtils.buildVimPlugin {
      name = "dpp-ext-installer";
      src = inputs.dpp-ext-installer;
    };
    dpp-ext-lazy = pkgs.vimUtils.buildVimPlugin {
      name = "dpp-ext-lazy";
      src = inputs.dpp-ext-lazy;
    };
    dpp-ext-toml = pkgs.vimUtils.buildVimPlugin {
      name = "dpp-ext-toml";
      src = inputs.dpp-ext-toml;
    };
    dpp-protocol-git = pkgs.vimUtils.buildVimPlugin {
      name = "dpp-protocol-git";
      src = inputs.dpp-protocol-git;
    };
  };
in
{
  programs.nixvim = {
    # This just enables NixVim.
    enable = true;

    # Load external Lua configuration
    extraConfigLua = builtins.readFile ./nvim.lua;

    # Use extraPlugins:
    extraPlugins = with pkgs.vimPlugins; [ 
      vim-toml 
    ] ++ (with dppPlugins; [
      # dpp plugin manager and extensions
      dpp-vim
      denops-vim
      dpp-ext-installer
      dpp-ext-lazy
      dpp-ext-toml
      dpp-protocol-git
    ]);
  };
}
