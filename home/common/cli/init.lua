-- Optimized Neovim configuration with Stylix theming

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

  -- Colorscheme is managed by Stylix (base16 Shonan theme)
  -- No explicit colorscheme plugin needed - Stylix injects colors automatically

  -- File explorer
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

  -- Fuzzy finder is provided by snacks.picker (see snacks.nvim block below)

  -- Status line (theme managed by Stylix)
  {
    "nvim-lualine/lualine.nvim",
    event = "VeryLazy",
    dependencies = { "nvim-tree/nvim-web-devicons" },
    config = function()
      require('lualine').setup({
        options = {
          theme = 'auto',
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

  -- Completion (blink.cmp — Rust-backed, replaces nvim-cmp + cmp-* sources)
  {
    "saghen/blink.cmp",
    version = "*",
    event = "InsertEnter",
    dependencies = { "rafamadriz/friendly-snippets" },
    opts = {
      keymap = {
        preset = "none",
        ["<C-Space>"] = { "show", "show_documentation", "hide_documentation" },
        ["<C-e>"] = { "hide", "fallback" },
        ["<CR>"] = { "accept", "fallback" },
        ["<Tab>"] = { "select_next", "fallback" },
        ["<S-Tab>"] = { "select_prev", "fallback" },
        ["<C-b>"] = { "scroll_documentation_up", "fallback" },
        ["<C-f>"] = { "scroll_documentation_down", "fallback" },
      },
      appearance = { nerd_font_variant = "mono" },
      completion = {
        menu = { border = "single" },
        documentation = {
          auto_show = true,
          window = { border = "single" },
        },
      },
      sources = {
        default = { "lazydev", "lsp", "path", "snippets", "buffer" },
        providers = {
          lazydev = {
            name = "LazyDev",
            module = "lazydev.integrations.blink",
            score_offset = 100,
          },
        },
      },
      signature = { enabled = true },
    },
  },

  -- Lua LSP enrichment for Neovim config (replaces lua_ls workspace.library hack)
  {
    "folke/lazydev.nvim",
    ft = "lua",
    opts = {
      library = {
        { path = "${3rd}/luv/library", words = { "vim%.uv" } },
      },
    },
  },

  -- LSP with proper highlighting
  {
    "neovim/nvim-lspconfig",
    event = { "BufReadPost", "BufNewFile" },
    dependencies = { "saghen/blink.cmp" },
    config = function()
      local capabilities = require('blink.cmp').get_lsp_capabilities()

      -- Configure LSP servers (Neovim 0.11+ API)
      local servers = {
        pyright = {
          settings = {
            pyright = {
              -- Use Ruff for organizing imports
              disableOrganizeImports = true,
            },
            python = {
              analysis = {
                -- Use Ruff for linting; pyright handles type checking only
                ignore = { '*' },
              },
            },
          },
        },
        ruff = {},
        ts_ls = {},
        rust_analyzer = {},
        gopls = {},
        lua_ls = {
          settings = {
            Lua = {
              runtime = { version = 'LuaJIT' },
              -- workspace.library handled by lazydev.nvim
              telemetry = { enable = false },
            },
          },
        },
      }

      for server, opts in pairs(servers) do
        opts.capabilities = capabilities
        vim.lsp.config(server, opts)
        vim.lsp.enable(server)
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
          vim.keymap.set({ 'n', 'v' }, '<leader>cf', function()
            vim.lsp.buf.format({ async = false })
          end, opts)
        end,
      })

      -- For Python: pyright handles hover/completion, ruff handles linting/formatting
      vim.api.nvim_create_autocmd('LspAttach', {
        group = vim.api.nvim_create_augroup('UserLspPython', {}),
        callback = function(args)
          local client = vim.lsp.get_client_by_id(args.data.client_id)
          if client and client.name == 'ruff' then
            -- Disable hover from Ruff in favor of Pyright
            client.server_capabilities.hoverProvider = false
          end
        end,
      })
    end,
  },

  -- GitHub Copilot (inline ghost-text suggestions)
  {
    "zbirenbaum/copilot.lua",
    event = "InsertEnter",
    config = function()
      require("copilot").setup({
        suggestion = {
          enabled = true,
          auto_trigger = true,
          hide_during_completion = true,
          keymap = {
            accept = "<M-l>",
            accept_word = "<M-w>",
            accept_line = "<M-j>",
            next = "<M-]>",
            prev = "<M-[>",
            dismiss = "<C-]>",
          },
        },
        panel = { enabled = false },
      })
    end,
  },

  -- Git integration
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

  -- Editor enhancements (gc/gcc commenting is built-in since Neovim 0.10)
  {
    "windwp/nvim-autopairs",
    event = "InsertEnter",
    opts = {
      check_ts = true,
      disable_filetype = { "vim" },
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

  -- Treesitter syntax highlighting (new API for nvim-treesitter 1.0+ / Neovim 0.11+)
  {
    "nvim-treesitter/nvim-treesitter",
    lazy = false,
    build = ":TSUpdate",
    config = function()
      require("nvim-treesitter").setup({
        ensure_installed = {
          "lua", "vim", "vimdoc", "query",
          "python", "javascript", "typescript", "tsx",
          "rust", "go", "c", "cpp",
          "json", "yaml", "toml", "markdown", "markdown_inline",
          "bash", "nix", "html", "css",
        },
        highlight = { enable = true },
        indent = { enable = true },
        auto_install = true,
      })

      vim.opt.foldmethod = "expr"
      vim.opt.foldexpr = "v:lua.vim.treesitter.foldexpr()"
      vim.opt.foldenable = false
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
      input = { enabled = true },  -- Required for opencode.nvim
      terminal = { enabled = true },  -- Required for opencode.nvim
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
          { icon = " ", key = "O", desc = "Open in Obsidian", action = ":Obsidian open", indent = 2 },
          { icon = " ", key = "N", desc = "New Note", action = ":Obsidian new", indent = 2 },
          { icon = " ", key = "S", desc = "Search Notes", action = ":Obsidian search", indent = 2 },
          { icon = " ", key = "T", desc = "Today's Note", action = ":Obsidian today", indent = 2 },
          { icon = " ", key = "D", desc = "Daily Notes", action = ":Obsidian dailies", indent = 2 },
          { icon = " ", key = "W", desc = "Switch Workspace", action = ":Obsidian workspace", indent = 2, padding = 1 },
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
      -- Picker (find prefix — replaces telescope keymaps)
      { "<leader>ff", function() Snacks.picker.files() end, desc = "Find Files" },
      { "<leader>fg", function() Snacks.picker.grep() end, desc = "Live Grep" },
      { "<leader>fb", function() Snacks.picker.buffers() end, desc = "Buffers" },
      { "<leader>fh", function() Snacks.picker.help() end, desc = "Help Tags" },
      { "<leader>fr", function() Snacks.picker.recent() end, desc = "Recent Files" },
      { "<leader>fc", function() Snacks.picker.commands() end, desc = "Commands" },
      { "<leader>fk", function() Snacks.picker.keymaps() end, desc = "Keymaps" },
      { "<leader>fs", function() Snacks.picker.lines() end, desc = "Search in Buffer" },
      { "<leader>fS", function() Snacks.picker.smart() end, desc = "Smart Picker" },
      { "<leader>gc", function() Snacks.picker.git_log() end, desc = "Git Commits" },
      { "<leader>gs", function() Snacks.picker.git_status() end, desc = "Git Status" },
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

  -- OpenCode integration (AI assistant)
  {
    "NickvanDyke/opencode.nvim",
    dependencies = {
      -- snacks.nvim is already configured above
      "folke/snacks.nvim",
    },
    config = function()
      ---@type opencode.Opts
      vim.g.opencode_opts = {
        -- Configuration options
      }

      -- Required for opts.events.reload
      vim.o.autoread = true

      -- Keymaps for opencode
      vim.keymap.set({ "n", "x" }, "<C-a>", function() require("opencode").ask("@this: ", { submit = true }) end, { desc = "Ask opencode" })
      vim.keymap.set({ "n", "x" }, "<C-x>", function() require("opencode").select() end, { desc = "Execute opencode action…" })
      vim.keymap.set({ "n", "t" }, "<C-.>", function() require("opencode").toggle() end, { desc = "Toggle opencode" })

      vim.keymap.set({ "n", "x" }, "go", function() return require("opencode").operator("@this ") end, { expr = true, desc = "Add range to opencode" })
      vim.keymap.set("n", "goo", function() return require("opencode").operator("@this ") .. "_" end, { expr = true, desc = "Add line to opencode" })

      vim.keymap.set("n", "<S-C-u>", function() require("opencode").command("session.half.page.up") end, { desc = "opencode half page up" })
      vim.keymap.set("n", "<S-C-d>", function() require("opencode").command("session.half.page.down") end, { desc = "opencode half page down" })

      -- Remap increment/decrement since we use <C-a> and <C-x> for opencode
      vim.keymap.set("n", "+", "<C-a>", { desc = "Increment", noremap = true })
      vim.keymap.set("n", "-", "<C-x>", { desc = "Decrement", noremap = true })
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
      vim.g.molten_image_provider = vim.fn.executable("wezterm") == 1 and "wezterm" or "none"
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

  -- WezTerm integration (only load if wezterm is installed)
  {
    "willothy/wezterm.nvim",
    cond = vim.fn.executable("wezterm") == 1,
    config = true,
  },

  -- Obsidian.nvim for note-taking (active fork — epwalsh/obsidian.nvim is archived)
  {
    "obsidian-nvim/obsidian.nvim",
    version = "*",
    lazy = true,
    ft = "markdown",
    cmd = { "Obsidian" },
    dependencies = {
      "nvim-lua/plenary.nvim",
    },
    opts = function()
      -- Use vaults from Nix configuration (vim.g.obsidian_vaults)
      local vaults = vim.g.obsidian_vaults or {
        { name = "Private", path = vim.fn.expand("~/Library/Mobile Documents/iCloud~md~obsidian/Documents/Private") },
        { name = "Work", path = vim.fn.expand("~/Library/Mobile Documents/iCloud~md~obsidian/Documents/Work") },
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

        -- Completion (use blink.cmp instead of nvim-cmp)
        completion = {
          nvim_cmp = false,
          blink = true,
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

        -- Picker for link suggestions (uses snacks.picker)
        picker = {
          name = "snacks.pick",
          note_mappings = {
            new = "<C-x>",
            insert_link = "<C-l>",
          },
        },
      }
    end,
    keys = {
      { "<leader>on", "<cmd>Obsidian new<cr>", desc = "New Obsidian note" },
      { "<leader>oo", "<cmd>Obsidian open<cr>", desc = "Open in Obsidian" },
      { "<leader>os", "<cmd>Obsidian search<cr>", desc = "Search Obsidian notes" },
      { "<leader>oq", "<cmd>Obsidian quick_switch<cr>", desc = "Quick switch note" },
      { "<leader>ob", "<cmd>Obsidian backlinks<cr>", desc = "Show backlinks" },
      { "<leader>ol", "<cmd>Obsidian links<cr>", desc = "Show links" },
      { "<leader>ot", "<cmd>Obsidian today<cr>", desc = "Today's daily note" },
      { "<leader>oy", "<cmd>Obsidian yesterday<cr>", desc = "Yesterday's daily note" },
      { "<leader>om", "<cmd>Obsidian tomorrow<cr>", desc = "Tomorrow's daily note" },
      { "<leader>od", "<cmd>Obsidian dailies<cr>", desc = "List daily notes" },
      { "<leader>ow", "<cmd>Obsidian workspace<cr>", desc = "Switch workspace" },
      { "<leader>op", "<cmd>Obsidian paste_img<cr>", desc = "Paste image" },
      { "<leader>or", "<cmd>Obsidian rename<cr>", desc = "Rename note" },
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
