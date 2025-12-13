-- Optimized Neovim configuration with Nord theme and proper lazy loading

-- Basic settings
vim.opt.tabstop = 4
vim.opt.shiftwidth = 4
vim.opt.expandtab = false
vim.opt.mouse = "a"
vim.opt.clipboard = "unnamedplus"
vim.opt.number = true
vim.opt.relativenumber = true
vim.opt.signcolumn = "yes"
vim.opt.cursorline = true
vim.opt.termguicolors = true
vim.g.mapleader = " "
vim.g.maplocalleader = "\\"

-- Nord color palette for consistency
local nord = {
  nord0 = "#2e3440",   -- Polar Night
  nord1 = "#3b4252",
  nord2 = "#434c5e",
  nord3 = "#4c566a",
  nord4 = "#d8dee9",   -- Snow Storm
  nord5 = "#e5e9f0",
  nord6 = "#eceff4",
  nord7 = "#8fbcbb",   -- Frost
  nord8 = "#88c0d0",
  nord9 = "#81a1c1",
  nord10 = "#5e81ac",
  nord11 = "#bf616a",  -- Aurora
  nord12 = "#d08770",
  nord13 = "#ebcb8b",
  nord14 = "#a3be8c",
  nord15 = "#b48ead",
}

-- Bootstrap lazy.nvim
local lazypath = vim.fn.stdpath("data") .. "/lazy/lazy.nvim"
if not vim.uv.fs_stat(lazypath) then
  vim.fn.system({
    "git", "clone", "--filter=blob:none",
    "https://github.com/folke/lazy.nvim.git",
    "--branch=stable", lazypath,
  })
end
vim.opt.rtp:prepend(lazypath)

-- Setup lazy.nvim with performance optimizations
require("lazy").setup({
  -- Performance optimization
  performance = {
    rtp = {
      disabled_plugins = {
        "gzip", "matchit", "matchparen", "netrwPlugin",
        "tarPlugin", "tohtml", "tutor", "zipPlugin",
      },
    },
  },

  -- Nord colorscheme - load immediately with high priority
  {
    "shaunsingh/nord.nvim",
    lazy = false,
    priority = 1000,
    config = function()
      vim.g.nord_contrast = true
      vim.g.nord_borders = false
      vim.g.nord_disable_background = false
      vim.g.nord_italic = false
      vim.g.nord_uniform_diff_background = true
      vim.g.nord_bold = false
      require('nord').set()
    end,
  },

  -- File explorer with Nord theme
  {
    "nvim-neo-tree/neo-tree.nvim",
    cmd = "Neotree",
    keys = {
      { "<leader>e", "<cmd>Neotree toggle<cr>", desc = "Toggle file explorer" },
      { "<leader>n", "<cmd>Neotree filesystem reveal left<cr>", desc = "Reveal in file explorer" },
      { "<leader>be", "<cmd>Neotree buffers reveal float<cr>", desc = "Show buffer explorer" },
    },
    dependencies = {
      "nvim-lua/plenary.nvim",
      "nvim-tree/nvim-web-devicons",
      "MunifTanjim/nui.nvim",
    },
    opts = {
      close_if_last_window = false,
      enable_git_status = true,
      enable_diagnostics = true,
      default_component_configs = {
        container = {
          enable_character_fade = true
        },
        indent = {
          indent_size = 2,
          padding = 1,
          with_markers = true,
          indent_marker = "│",
          last_indent_marker = "└",
          highlight = "NeoTreeIndentMarker",
          with_expanders = nil,
          expander_collapsed = "",
          expander_expanded = "",
          expander_highlight = "NeoTreeExpander",
        },
        icon = {
          folder_closed = "",
          folder_open = "",
          folder_empty = "󰜌",
          default = "*",
          highlight = "NeoTreeFileIcon"
        },
        modified = {
          symbol = "[+]",
          highlight = "NeoTreeModified",
        },
        name = {
          trailing_slash = false,
          use_git_status_colors = true,
          highlight = "NeoTreeFileName",
        },
        git_status = {
          symbols = {
            added     = "",
            modified  = "",
            deleted   = "✖",
            renamed   = "󰁕",
            untracked = "",
            ignored   = "",
            unstaged  = "󰄱",
            staged    = "",
            conflict  = "",
          }
        },
      },
      window = {
        position = "left",
        width = 40,
        mapping_options = {
          noremap = true,
          nowait = true,
        },
      },
      filesystem = {
        follow_current_file = {
          enabled = true,
        },
        use_libuv_file_watcher = true,
        filtered_items = {
          visible = false,
          hide_dotfiles = false,
          hide_gitignored = true,
        },
      },
    },
  },

  -- Fuzzy finder with Nord theme
  {
    "nvim-telescope/telescope.nvim",
    keys = {
      { "<leader>ff", "<cmd>Telescope find_files<cr>", desc = "Find files" },
      { "<leader>fg", "<cmd>Telescope live_grep<cr>", desc = "Live grep" },
      { "<leader>fb", "<cmd>Telescope buffers<cr>", desc = "Buffers" },
      { "<leader>fh", "<cmd>Telescope help_tags<cr>", desc = "Help tags" },
      { "<leader>fr", "<cmd>Telescope oldfiles<cr>", desc = "Recent files" },
      { "<leader>fc", "<cmd>Telescope commands<cr>", desc = "Commands" },
      { "<leader>fk", "<cmd>Telescope keymaps<cr>", desc = "Keymaps" },
      { "<leader>fs", "<cmd>Telescope current_buffer_fuzzy_find<cr>", desc = "Search in buffer" },
      { "<leader>gc", "<cmd>Telescope git_commits<cr>", desc = "Git commits" },
      { "<leader>gb", "<cmd>Telescope git_branches<cr>", desc = "Git branches" },
      { "<leader>gs", "<cmd>Telescope git_status<cr>", desc = "Git status" },
    },
    dependencies = { "nvim-lua/plenary.nvim" },
    config = function()
      local telescope = require('telescope')
      local actions = require('telescope.actions')

      telescope.setup({
        defaults = {
          prompt_prefix = "🔍 ",
          selection_caret = " ",
          entry_prefix = "  ",
          initial_mode = "insert",
          selection_strategy = "reset",
          sorting_strategy = "descending",
          layout_strategy = "horizontal",
          layout_config = {
            horizontal = {
              mirror = false,
            },
            vertical = {
              mirror = false,
            },
          },
          file_ignore_patterns = { "node_modules", ".git/", "%.lock" },
          winblend = 0,
          borderchars = { "─", "│", "─", "│", "╭", "╮", "╯", "╰" },
          color_devicons = true,
          use_less = true,
          path_display = { "truncate" },
          set_env = { ["COLORTERM"] = "truecolor" },
          mappings = {
            i = {
              ["<C-j>"] = actions.move_selection_next,
              ["<C-k>"] = actions.move_selection_previous,
              ["<C-n>"] = actions.cycle_history_next,
              ["<C-p>"] = actions.cycle_history_prev,
              ["<C-c>"] = actions.close,
              ["<Down>"] = actions.move_selection_next,
              ["<Up>"] = actions.move_selection_previous,
              ["<CR>"] = actions.select_default,
              ["<C-x>"] = actions.select_horizontal,
              ["<C-v>"] = actions.select_vertical,
              ["<C-t>"] = actions.select_tab,
              ["<C-u>"] = actions.preview_scrolling_up,
              ["<C-d>"] = actions.preview_scrolling_down,
            },
            n = {
              ["<esc>"] = actions.close,
              ["<CR>"] = actions.select_default,
              ["<C-x>"] = actions.select_horizontal,
              ["<C-v>"] = actions.select_vertical,
              ["<C-t>"] = actions.select_tab,
              ["j"] = actions.move_selection_next,
              ["k"] = actions.move_selection_previous,
              ["H"] = actions.move_to_top,
              ["M"] = actions.move_to_middle,
              ["L"] = actions.move_to_bottom,
              ["<C-u>"] = actions.preview_scrolling_up,
              ["<C-d>"] = actions.preview_scrolling_down,
            },
          },
        },
        pickers = {
          find_files = {
            theme = "dropdown",
            previewer = false,
          },
          live_grep = {
            theme = "ivy",
          },
          buffers = {
            theme = "dropdown",
            previewer = false,
            initial_mode = "normal",
          },
        },
        extensions = {},
      })
    end,
  },

  -- Status line with Nord theme
  {
    "nvim-lualine/lualine.nvim",
    event = "VeryLazy",
    dependencies = { "nvim-tree/nvim-web-devicons" },
    config = function()
      require('lualine').setup({
        options = {
          theme = 'nord',
          component_separators = { left = '', right = ''},
          section_separators = { left = '', right = ''},
          disabled_filetypes = {
            statusline = {},
            winbar = {},
          },
          ignore_focus = {},
          always_divide_middle = true,
          globalstatus = false,
          refresh = {
            statusline = 1000,
            tabline = 1000,
            winbar = 1000,
          }
        },
        sections = {
          lualine_a = {'mode'},
          lualine_b = {'branch', 'diff', 'diagnostics'},
          lualine_c = {'filename'},
          lualine_x = {'encoding', 'fileformat', 'filetype'},
          lualine_y = {'progress'},
          lualine_z = {'location'}
        },
        inactive_sections = {
          lualine_a = {},
          lualine_b = {},
          lualine_c = {'filename'},
          lualine_x = {'location'},
          lualine_y = {},
          lualine_z = {}
        },
        tabline = {},
        winbar = {},
        inactive_winbar = {},
        extensions = {}
      })
    end,
  },

  -- Completion with Nord-compatible styling
  {
    "hrsh7th/nvim-cmp",
    event = "InsertEnter",
    dependencies = {
      "hrsh7th/cmp-nvim-lsp",
      "hrsh7th/cmp-buffer",
      "hrsh7th/cmp-path",
    },
    config = function()
      local cmp = require("cmp")
      cmp.setup({
        window = {
          completion = cmp.config.window.bordered(),
          documentation = cmp.config.window.bordered(),
        },
        mapping = cmp.mapping.preset.insert({
          ['<C-b>'] = cmp.mapping.scroll_docs(-4),
          ['<C-f>'] = cmp.mapping.scroll_docs(4),
          ['<C-Space>'] = cmp.mapping.complete(),
          ['<C-e>'] = cmp.mapping.abort(),
          ['<CR>'] = cmp.mapping.confirm({ select = true }),
          ['<Tab>'] = cmp.mapping(function(fallback)
            if cmp.visible() then
              cmp.select_next_item()
            else
              fallback()
            end
          end, { 'i', 's' }),
          ['<S-Tab>'] = cmp.mapping(function(fallback)
            if cmp.visible() then
              cmp.select_prev_item()
            else
              fallback()
            end
          end, { 'i', 's' }),
        }),
        sources = cmp.config.sources({
          { name = 'nvim_lsp' },
          { name = 'copilot' },
        }, {
          { name = 'buffer' },
          { name = 'path' },
        }),
        formatting = {
          format = function(entry, vim_item)
            vim_item.menu = ({
              nvim_lsp = "[LSP]",
              copilot = "[Copilot]",
              buffer = "[Buffer]",
              path = "[Path]",
            })[entry.source.name]
            return vim_item
          end,
        },
      })
    end,
  },

  -- LSP with proper highlighting
  {
    "neovim/nvim-lspconfig",
    event = { "BufReadPost", "BufNewFile" },
    dependencies = { "hrsh7th/cmp-nvim-lsp" },
    config = function()
      local lspconfig = require('lspconfig')
      local capabilities = require('cmp_nvim_lsp').default_capabilities()

      -- Configure LSP servers
      local servers = {
        pyright = {},
        ts_ls = {},
        rust_analyzer = {},
        gopls = {},
        lua_ls = {
          settings = {
            Lua = {
              diagnostics = { globals = { 'vim' } },
              workspace = { checkThirdParty = false },
              telemetry = { enable = false },
            },
          },
        },
      }

      for server, opts in pairs(servers) do
        opts.capabilities = capabilities
        vim.lsp.config[server] = opts
      end

      -- LSP keymaps
      vim.api.nvim_create_autocmd('LspAttach', {
        group = vim.api.nvim_create_augroup('UserLspConfig', {}),
        callback = function(ev)
          local opts = { buffer = ev.buf }
          vim.keymap.set('n', 'gD', vim.lsp.buf.declaration, opts)
          vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
          vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
          vim.keymap.set('n', 'gi', vim.lsp.buf.implementation, opts)
          vim.keymap.set('n', '<C-k>', vim.lsp.buf.signature_help, opts)
          vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
          vim.keymap.set({ 'n', 'v' }, '<leader>ca', vim.lsp.buf.code_action, opts)
          vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
        end,
      })
    end,
  },

  -- GitHub Copilot
  {
    "zbirenbaum/copilot.lua",
    event = "InsertEnter",
    config = function()
      require("copilot").setup({
        suggestion = { enabled = false },
        panel = { enabled = false },
      })
    end,
  },

  {
    "zbirenbaum/copilot-cmp",
    event = "InsertEnter",
    dependencies = { "copilot.lua", "nvim-cmp" },
    config = function()
      require("copilot_cmp").setup()
    end,
  },

  -- Git integration with Nord-compatible signs
  {
    "lewis6991/gitsigns.nvim",
    event = { "BufReadPost", "BufNewFile" },
    opts = {
      signs = {
        add = { text = '│' },
        change = { text = '│' },
        delete = { text = '_' },
        topdelete = { text = '‾' },
        changedelete = { text = '~' },
        untracked = { text = '┆' },
      },
      signcolumn = true,
      numhl = false,
      linehl = false,
      word_diff = false,
      watch_gitdir = {
        interval = 1000,
        follow_files = true
      },
      attach_to_untracked = true,
      current_line_blame = false,
      current_line_blame_opts = {
        virt_text = true,
        virt_text_pos = 'eol',
        delay = 1000,
        ignore_whitespace = false,
      },
      preview_config = {
        border = 'single',
        style = 'minimal',
        relative = 'cursor',
        row = 0,
        col = 1
      },
    },
  },

  -- Editor enhancements
  {
    "numToStr/Comment.nvim",
    event = "VeryLazy",
    config = true,
  },

  {
    "windwp/nvim-autopairs",
    event = "InsertEnter",
    opts = {
      check_ts = true,
      disable_filetype = { "TelescopePrompt", "vim" },
      fast_wrap = {
        map = '<M-e>',
        chars = { '{', '[', '(', '"', "'" },
        pattern = string.gsub([[ [%'%"%)%>%]%)%}%,] ]], '%s+', ''),
        offset = 0,
        end_key = '$',
        keys = 'qwertyuiopzxcvbnmasdfghjkl',
        check_comma = true,
        highlight = 'PmenuSel',
        highlight_grey = 'LineNr'
      },
    },
  },

  -- Treesitter with Nord-compatible highlighting
  {
    "nvim-treesitter/nvim-treesitter",
    event = { "BufReadPost", "BufNewFile" },
    build = ":TSUpdate",
    config = function()
      require("nvim-treesitter.configs").setup({
        auto_install = true,
        highlight = {
          enable = true,
          additional_vim_regex_highlighting = false,
        },
        indent = { enable = true },
        incremental_selection = {
          enable = true,
          keymaps = {
            init_selection = "gnn",
            node_incremental = "grn",
            scope_incremental = "grc",
            node_decremental = "grm",
          },
        },
      })
    end,
  },

  -- Snacks.nvim - Collection of useful utilities
  {
    "folke/snacks.nvim",
    priority = 1000,
    lazy = false,
    dependencies = { "nvim-tree/nvim-web-devicons" },
    opts = {
      bigfile = { enabled = true },
      dashboard = {
        enabled = true,
        preset = {
          keys = {
            { icon = " ", key = "f", desc = "Find File", action = ":lua Snacks.dashboard.pick('files')" },
            { icon = " ", key = "n", desc = "New File", action = ":ene | startinsert" },
            { icon = " ", key = "g", desc = "Find Text", action = ":lua Snacks.dashboard.pick('live_grep')" },
            { icon = " ", key = "r", desc = "Recent Files", action = ":lua Snacks.dashboard.pick('oldfiles')" },
            { icon = " ", key = "c", desc = "Config", action = ":lua Snacks.dashboard.pick('files', {cwd = vim.fn.stdpath('config')})" },
            { icon = "󰒲 ", key = "L", desc = "Lazy", action = ":Lazy", enabled = package.loaded.lazy ~= nil },
            { icon = " ", key = "q", desc = "Quit", action = ":qa" },
          },
        },
        sections = {
          { section = "header" },
          { section = "keys", gap = 1, padding = 1 },
          { icon = " ", title = "Obsidian", padding = 1 },
          { icon = " ", key = "O", desc = "Open in Obsidian", action = ":ObsidianOpen", indent = 2 },
          { icon = " ", key = "N", desc = "New Note", action = ":ObsidianNew", indent = 2 },
          { icon = " ", key = "S", desc = "Search Notes", action = ":ObsidianSearch", indent = 2 },
          { icon = " ", key = "T", desc = "Today's Note", action = ":ObsidianToday", indent = 2 },
          { icon = " ", key = "D", desc = "Daily Notes", action = ":ObsidianDailies", indent = 2 },
          { icon = " ", key = "W", desc = "Switch Workspace", action = ":ObsidianWorkspace", indent = 2, padding = 1 },
          { icon = " ", title = "Recent Files", section = "recent_files", cwd = true, limit = 5, indent = 2, padding = 1 },
          { section = "startup" },
        },
      },
      notifier = { enabled = true },
      quickfile = { enabled = true },
      statuscolumn = { enabled = true },
      words = { enabled = true },
      lazygit = { enabled = true },
      git = { enabled = true },
      gitbrowse = { enabled = true },
      zen = { enabled = true },
      scroll = { enabled = true },
      indent = { enabled = true },
      animate = { enabled = true },
      picker = { enabled = true },
      explorer = { enabled = true },
    },
    keys = {
      { "<leader>z", function() Snacks.zen() end, desc = "Toggle Zen Mode" },
      { "<leader>Z", function() Snacks.zen.zoom() end, desc = "Toggle Zoom" },
      { "<leader>gg", function() Snacks.lazygit() end, desc = "Lazygit" },
      { "<leader>gb", function() Snacks.git.blame_line() end, desc = "Git Blame Line" },
      { "<leader>gB", function() Snacks.gitbrowse() end, desc = "Git Browse" },
      { "<leader>gf", function() Snacks.lazygit.log_file() end, desc = "Lazygit Current File History" },
      { "<leader>gl", function() Snacks.lazygit.log() end, desc = "Lazygit Log (cwd)" },
      { "<leader>cR", function() Snacks.rename.rename_file() end, desc = "Rename File" },
      { "<leader>gY", function() Snacks.gitbrowse.open({ what = "repo" }) end, desc = "Open Repo URL" },
      { "<c-/>", function() Snacks.terminal() end, desc = "Toggle Terminal" },
      { "<c-_>", function() Snacks.terminal() end, desc = "Toggle Terminal (which-key shows this)" },
      { "]]", function() Snacks.words.jump(vim.v.count1) end, desc = "Next Reference", mode = { "n", "t" } },
      { "[[", function() Snacks.words.jump(-vim.v.count1) end, desc = "Prev Reference", mode = { "n", "t" } },
      { "<leader>bd", function() Snacks.bufdelete() end, desc = "Delete Buffer" },
      { "<leader>bo", function() Snacks.bufdelete.other() end, desc = "Delete Other Buffers" },
      { "<leader>ba", function() Snacks.bufdelete.all() end, desc = "Delete All Buffers" },
      { "<leader>N", function() Snacks.notifier.show_history() end, desc = "Notification History" },
      { "<leader>un", function() Snacks.notifier.hide() end, desc = "Dismiss All Notifications" },
      { "<leader>ps", function() Snacks.picker.smart() end, desc = "Smart Picker" },
      { "<leader>pf", function() Snacks.picker.files() end, desc = "Find Files" },
      { "<leader>pg", function() Snacks.picker.grep() end, desc = "Grep" },
      { "<leader>pb", function() Snacks.picker.buffers() end, desc = "Buffers" },
      { "<leader>ph", function() Snacks.picker.help() end, desc = "Help" },
      { "<leader>pr", function() Snacks.picker.recent() end, desc = "Recent Files" },
      { "<leader>pc", function() Snacks.picker.commands() end, desc = "Commands" },
      { "<leader>pk", function() Snacks.picker.keymaps() end, desc = "Keymaps" },
      { "<leader>pgs", function() Snacks.picker.git_status() end, desc = "Git Status" },
      { "<leader>pgc", function() Snacks.picker.git_log() end, desc = "Git Commits" },
      { "<leader>se", function() Snacks.explorer() end, desc = "Toggle Explorer" },
    },
    config = function(_, opts)
      require("snacks").setup(opts)

      -- Create autocmd for snacks dashboard
      vim.api.nvim_create_autocmd("User", {
        pattern = "VeryLazy",
        callback = function()
          -- Show the dashboard when starting Neovim with no arguments
          if vim.o.filetype == "" and vim.api.nvim_buf_line_count(0) == 1 and vim.api.nvim_buf_get_lines(0, 0, -1, false)[1] == "" then
            require("snacks").dashboard()
          end
        end,
      })
    end,
  },

  -- Claude Code integration
  {
    "coder/claudecode.nvim",
    cmd = { "ClaudeCode" },
    keys = {
      { "<leader>cc", "<cmd>ClaudeCode<cr>", desc = "Open Claude Code" },
    },
    config = function()
      require("claudecode").setup({
        -- Configuration options (if any)
      })
    end,
  },

  -- Jupytext for notebook conversion and rendering
  {
    "GCBallesteros/jupytext.nvim",
    config = function()
      require("jupytext").setup({
        style = "markdown",
        output_extension = "md",
        force_ft = "markdown",
      })
    end,
    -- Lazy load on .ipynb files
    ft = { "ipynb" },
  },

  -- Jupyter notebook ecosystem
  {
    "benlubas/molten-nvim",
    version = "^1.0.0",
    build = ":UpdateRemotePlugins",
    dependencies = { "GCBallesteros/jupytext.nvim" },
    init = function()
      -- Configuration for molten with WezTerm image provider
      vim.g.molten_output_win_max_height = 20
      vim.g.molten_auto_open_output = false
      vim.g.molten_wrap_output = true
      vim.g.molten_virt_text_output = true
      vim.g.molten_virt_lines_off_by_1 = true
      vim.g.molten_image_provider = "wezterm"
      vim.g.molten_split_direction = "right"
      vim.g.molten_split_size = 40
    end,
    keys = {
      { "<leader>mi", ":MoltenInit<CR>", desc = "Initialize Molten" },
      { "<leader>e", ":MoltenEvaluateOperator<CR>", desc = "Evaluate operator", mode = "n" },
      { "<leader>r", ":<C-u>MoltenEvaluateVisual<CR>gv", desc = "Evaluate visual", mode = "v" },
      { "<leader>rr", ":MoltenReevaluateCell<CR>", desc = "Re-evaluate cell" },
      { "<leader>os", ":noautocmd MoltenEnterOutput<CR>", desc = "Enter output window" },
      { "<leader>oh", ":MoltenHideOutput<CR>", desc = "Hide output" },
      { "<leader>md", ":MoltenDelete<CR>", desc = "Delete cell" },
    },
    ft = { "ipynb", "markdown" },
  },

  -- Quarto support for mixed notebook/document editing
  {
    "quarto-dev/quarto-nvim",
    ft = { "quarto", "markdown" },
    dependencies = {
      "jmbuhr/otter.nvim",
      "nvim-treesitter/nvim-treesitter",
    },
    opts = {
      lspFeatures = {
        enabled = true,
        languages = { "python", "bash", "html" },
        chunks = "curly",
        diagnostics = {
          enabled = true,
          triggers = { "BufWritePost" },
        },
        completion = {
          enabled = true,
        },
      },
      codeRunner = {
        enabled = true,
        default_method = "molten",
      },
    },
  },

  -- Otter for embedded language support
  {
    "jmbuhr/otter.nvim",
    ft = { "quarto", "markdown" },
    opts = {
      lsp = {
        hover = {
          border = "rounded",
        },
      },
      buffers = {
        set_filetype = true,
      },
      handle_leading_whitespace = true,
    },
  },

  -- WezTerm integration
  {
    "willothy/wezterm.nvim",
    config = true,
  },

  -- Obsidian.nvim for note-taking
  {
    "epwalsh/obsidian.nvim",
    version = "*",
    lazy = true,
    ft = "markdown",
    cmd = {
      "ObsidianOpen",
      "ObsidianNew",
      "ObsidianSearch",
      "ObsidianQuickSwitch",
      "ObsidianToday",
      "ObsidianYesterday",
      "ObsidianTomorrow",
      "ObsidianDailies",
      "ObsidianWorkspace",
      "ObsidianBacklinks",
      "ObsidianLinks",
      "ObsidianPasteImg",
      "ObsidianRename",
    },
    dependencies = {
      "nvim-lua/plenary.nvim",
    },
    opts = function()
      -- Use vaults from Nix configuration (vim.g.obsidian_vaults)
      local vaults = vim.g.obsidian_vaults or {
        { name = "Main", path = vim.fn.expand("~/Obsidian/Main") }
      }

      -- Convert to obsidian.nvim format
      local workspaces = {}
      for _, vault in ipairs(vaults) do
        table.insert(workspaces, {
          name = vault.name,
          path = vault.path,
        })
      end

      return {
        workspaces = workspaces,

        -- Note formatting
        notes_subdir = "notes",
        new_notes_location = "notes_subdir",

        -- Daily notes configuration
        daily_notes = {
          folder = "daily",
          date_format = "%Y-%m-%d",
          template = nil,
        },

        -- Completion
        completion = {
          nvim_cmp = true,
          min_chars = 2,
        },

        -- Note ID generation
        note_id_func = function(title)
          local suffix = ""
          if title ~= nil then
            suffix = title:gsub(" ", "-"):gsub("[^A-Za-z0-9-]", ""):lower()
          else
            for _ = 1, 4 do
              suffix = suffix .. string.char(math.random(65, 90))
            end
          end
          return tostring(os.date("%Y%m%d%H%M")) .. "-" .. suffix
        end,

        -- UI settings
        ui = {
          enable = true,
          update_debounce = 200,
          checkboxes = {
            [" "] = { char = "󰄱", hl_group = "ObsidianTodo" },
            ["x"] = { char = "", hl_group = "ObsidianDone" },
            [">"] = { char = "", hl_group = "ObsidianRightArrow" },
            ["~"] = { char = "󰰱", hl_group = "ObsidianTilde" },
          },
          bullets = { char = "•", hl_group = "ObsidianBullet" },
          external_link_icon = { char = "", hl_group = "ObsidianExtLinkIcon" },
          reference_text = { hl_group = "ObsidianRefText" },
          highlight_text = { hl_group = "ObsidianHighlightText" },
          tags = { hl_group = "ObsidianTag" },
          hl_groups = {
            ObsidianTodo = { bold = true, fg = "#f78c6c" },
            ObsidianDone = { bold = true, fg = "#89ddff" },
            ObsidianRightArrow = { bold = true, fg = "#f78c6c" },
            ObsidianTilde = { bold = true, fg = "#ff5370" },
            ObsidianBullet = { bold = true, fg = "#89ddff" },
            ObsidianRefText = { underline = true, fg = "#c792ea" },
            ObsidianExtLinkIcon = { fg = "#c792ea" },
            ObsidianTag = { italic = true, fg = "#89ddff" },
            ObsidianHighlightText = { bg = "#75662e" },
          },
        },

        -- Mappings
        mappings = {
          ["gf"] = {
            action = function()
              return require("obsidian").util.gf_passthrough()
            end,
            opts = { noremap = false, expr = true, buffer = true },
          },
          ["<leader>ch"] = {
            action = function()
              return require("obsidian").util.toggle_checkbox()
            end,
            opts = { buffer = true },
          },
        },

        -- Picker for link suggestions
        picker = {
          name = "telescope.nvim",
          mappings = {
            new = "<C-x>",
            insert_link = "<C-l>",
          },
        },
      }
    end,
    keys = {
      { "<leader>on", "<cmd>ObsidianNew<cr>", desc = "New Obsidian note" },
      { "<leader>oo", "<cmd>ObsidianOpen<cr>", desc = "Open in Obsidian" },
      { "<leader>os", "<cmd>ObsidianSearch<cr>", desc = "Search Obsidian notes" },
      { "<leader>oq", "<cmd>ObsidianQuickSwitch<cr>", desc = "Quick switch note" },
      { "<leader>ob", "<cmd>ObsidianBacklinks<cr>", desc = "Show backlinks" },
      { "<leader>ol", "<cmd>ObsidianLinks<cr>", desc = "Show links" },
      { "<leader>ot", "<cmd>ObsidianToday<cr>", desc = "Today's daily note" },
      { "<leader>oy", "<cmd>ObsidianYesterday<cr>", desc = "Yesterday's daily note" },
      { "<leader>om", "<cmd>ObsidianTomorrow<cr>", desc = "Tomorrow's daily note" },
      { "<leader>od", "<cmd>ObsidianDailies<cr>", desc = "List daily notes" },
      { "<leader>ow", "<cmd>ObsidianWorkspace<cr>", desc = "Switch workspace" },
      { "<leader>op", "<cmd>ObsidianPasteImg<cr>", desc = "Paste image" },
      { "<leader>or", "<cmd>ObsidianRename<cr>", desc = "Rename note" },
    },
  },

  -- Markview.nvim for markdown preview/rendering
  {
    "OXY2DEV/markview.nvim",
    lazy = false,
    dependencies = {
      "nvim-treesitter/nvim-treesitter",
      "nvim-tree/nvim-web-devicons",
    },
    opts = {
      modes = { "n", "no", "c" },
      hybrid_modes = { "n" },
      callbacks = {
        on_enable = function(_, win)
          vim.wo[win].conceallevel = 2
          vim.wo[win].concealcursor = "c"
        end,
      },
    },
    keys = {
      { "<leader>mt", "<cmd>Markview toggleAll<cr>", desc = "Toggle Markview" },
      { "<leader>ms", "<cmd>Markview splitToggle<cr>", desc = "Toggle split preview" },
    },
  },

  -- Jupyter notebook autocommands and configuration
  {
    "benlubas/molten-nvim",
    ft = "ipynb",
    config = function()
      -- Autocommands for notebook handling
      local augroup = vim.api.nvim_create_augroup("MoltenNotebook", { clear = true })

      -- Import output chunks when opening .ipynb files
      local function imb()
        vim.schedule(function()
          if vim.fn.executable("jupyter") == 1 then
            vim.cmd("MoltenInit")
            vim.cmd("MoltenImportOutput")
          end
        end)
      end

      vim.api.nvim_create_autocmd("BufAdd", {
        group = augroup,
        pattern = "*.ipynb",
        callback = imb,
      })

      -- Export output chunks when saving .ipynb files
      vim.api.nvim_create_autocmd("BufWritePost", {
        group = augroup,
        pattern = "*.ipynb",
        callback = function()
          if require("molten.status").initialized() == "Molten" then
            vim.cmd("MoltenExportOutput!")
          end
        end,
      })
    end,
  },

  -- Library dependencies (lazy loaded automatically)
  { "nvim-lua/plenary.nvim", lazy = true },
  { "nvim-tree/nvim-web-devicons", lazy = true },
  { "MunifTanjim/nui.nvim", lazy = true },
}, {
  ui = {
    border = "rounded",
  },
  change_detection = {
    notify = false,
  },
})

-- Filetype detection for Jupyter notebooks
vim.filetype.add({
  extension = {
    ipynb = 'ipynb',
  },
})

-- Essential keymaps
vim.keymap.set('n', '<leader>w', '<cmd>w<cr>', { desc = "Save file" })
vim.keymap.set('n', '<leader>q', '<cmd>q<cr>', { desc = "Quit" })
vim.keymap.set('n', '<leader>wq', '<cmd>wq<cr>', { desc = "Save and quit" })
vim.keymap.set('n', '<esc><esc>', '<cmd>nohlsearch<cr><esc>', { desc = "Clear search" })

-- Navigation
vim.keymap.set('n', 'j', 'gj', { desc = "Move down (visual line)" })
vim.keymap.set('n', 'k', 'gk', { desc = "Move up (visual line)" })
vim.keymap.set('n', '<S-h>', '^', { desc = "Move to line start" })
vim.keymap.set('n', '<S-l>', '$', { desc = "Move to line end" })

-- Gitsigns keymaps (loaded after gitsigns plugin)
vim.api.nvim_create_autocmd("User", {
  pattern = "LazyLoad",
  callback = function(event)
    if event.data == "gitsigns.nvim" then
      local gs = require('gitsigns')
      vim.keymap.set('n', ']c', function() gs.next_hunk() end, { desc = 'Next git hunk' })
      vim.keymap.set('n', '[c', function() gs.prev_hunk() end, { desc = 'Previous git hunk' })
      vim.keymap.set('n', '<leader>hs', gs.stage_hunk, { desc = 'Stage hunk' })
      vim.keymap.set('n', '<leader>hr', gs.reset_hunk, { desc = 'Reset hunk' })
      vim.keymap.set('n', '<leader>hp', gs.preview_hunk, { desc = 'Preview hunk' })
      vim.keymap.set('n', '<leader>hb', gs.blame_line, { desc = 'Blame line' })
      vim.keymap.set('n', '<leader>tb', gs.toggle_current_line_blame, { desc = 'Toggle line blame' })
      vim.keymap.set('n', '<leader>hd', gs.diffthis, { desc = 'Diff this' })
      vim.keymap.set('n', '<leader>td', gs.toggle_deleted, { desc = 'Toggle deleted' })
    end
  end,
})
