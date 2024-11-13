{ pkgs, ... }:
{
  programs.nixvim = {
    # This just enables NixVim.
    # If all you have is this, then there will be little visible difference
    # when compared to just installing NeoVim.
    enable = true;

    keymaps = [
      # Equivalent to nmap <silent> <buffer> <leader>gg <cmd>Man<CR>
      {
        key = "<leader>gg";
        action = "<cmd>Man<CR>";
        options = {
          silent = true;
          remap = false;
        };
      }
      {
        key = "j";
        action = "gj";
        options = {
          silent = true;
          remap = false;
        };
      }
      {
        key = "k";
        action = "gk";
        options = {
          silent = true;
          remap = false;
        };
      }
      {
        key = "<S-h>";
        action = "^";
        options = {
          silent = true;
          remap = false;
        };
      }
      {
        key = "<S-l>";
        action = "$";
        options = {
          silent = true;
          remap = false;
		};
      }

      {
        key = "<S-k>";
        action = "{";
        options = {
          silent = true;
          remap = false;
        };
      }
      {
        key = "<S-j>";
        action = "}";
        options = {
          silent = true;
          remap = false;
        };
      }
      {
        key = "m";
        action = "%";
        options = {
          silent = true;
          remap = false;
        };
      }
      {
        key = "<leader>w";
        action = ":w<CR>";
        options = {
          silent = true;
          remap = true;
        };
      }
      {
        key = "<leader>q";
        action = ":q<CR>";
        options = {
          silent = true;
          remap = true;
        };
      }
      {
        key = "<leader>wq";
        action = ":wq<CR>";
        options = {
          silent = true;
          remap = true;
        };
      }
      {
        key = "<esc><esc>";
        action = ":nohlsearch<CR><esc>";
        options = {
          silent = true;
          remap = true;
        };
      }
      # Plugins
      {
        key = "<leader>n";
        action = ":Neotree filesystem reveal left<CR>";
        options = {
          silent = true;
          remap = true;
        };
      }
    ];

    # We can set the leader key:
    globals.mapleader = " ";

    # We can create maps for every mode!
    # There is .normal, .insert, .visual, .operator, etc!

    # We can also set options:
    opts = {
      tabstop = 4;
      shiftwidth = 4;
      expandtab = false;

      mouse = "a";

      # etc...
    };

    # Of course, we can still use comfy vimscript:
    # extraConfigVim = builtins.readFile ./init.vim;
    # Or lua!
    # extraConfigLua = builtins.readFile ./init.lua;

    # One of the big advantages of NixVim is how it provides modules for
    # popular vim plugins
    # Enabling a plugin this way skips all the boring configuration that
    # some plugins tend to require.
    plugins = {
      lightline = {
        enable = true;

        # This is optional - it will default to your enabled colorscheme
        settings = {
          colorscheme = "wombat";

          # This is one of lightline's example configurations
          active = {
            left = [
              [
                "mode"
                "paste"
              ]
              [
                "readonly"
                "filename"
                "modified"
                "helloworld"
              ]
            ];
          };

          component = {
            helloworld = "Hello, world!";
          };
        };
      };

	  lazy = {
	    enable = true;
	  };
	  nvim-autopairs = {
	    enable = true;
	  };
	  comment = {
	    enable = true;
	  };
	  nvim-colorizer = {
	    enable = true;
	  };
	  cmp = {
	    enable = true;
	  };
	  copilot-cmp = {
	    enable = true;
	  };
	  copilot-lua = {
	    enable = true;
		suggestion = {
		  enabled = false;
		};
		panel = {
		  enabled = false;
		};
	  };
	  telescope = {
        enable = true;
	  };
	  web-devicons = {
	    enable = true;
	  };
	  neo-tree = {
	    enable = true;
	  };
	  treesitter = {
	    enable = true;
	  };



      # Of course, there are a lot more plugins available.
      # You can find an up-to-date list here:
      # https://nixvim.pta2002.com/plugins
    };

	extraConfigLua = ''
		require("copilot").setup({
		suggestion = { enabled = false },
		panel = { enabled = false },
		})
	'';

    # There is a separate namespace for colorschemes:
    colorschemes.nord.enable = true;

    # What about plugins not available as a module?
    # Use extraPlugins:
    extraPlugins = with pkgs.vimPlugins; [ vim-toml ];
  };
}
