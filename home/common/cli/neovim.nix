{ pkgs, ... }:
{
  programs.nixvim = {
    enable = true;

    # Load external Lua configuration (init.lua)
    extraConfigLua = builtins.readFile ./init.lua;

    # Add lazy.nvim plugin to be available for bootstrapping
    extraPlugins = with pkgs.vimPlugins; [
      lazy-nvim
    ];

    # Enable lazy loading support in nixvim
    plugins = {
      # Disable nixvim's built-in package management to let lazy.nvim handle it
      lazy = {
        enable = true;
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
}