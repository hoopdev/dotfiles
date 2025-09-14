-- Neovim configuration
-- Neovim configuration with dpp plugin manager
local M = {}

-- Basic Neovim options (moved from nixvim opts)
vim.opt.tabstop = 4
vim.opt.shiftwidth = 4
vim.opt.expandtab = false
vim.opt.mouse = "a"
vim.opt.clipboard = "unnamedplus"

-- Set leader key (moved from nixvim globals)
vim.g.mapleader = " "

-- Basic keymaps (moved from nixvim keymaps)
local keymap = vim.keymap.set
local opts = { silent = true, remap = false }
local remap_opts = { silent = true, remap = true }

-- Navigation keymaps
keymap('n', '<leader>gg', '<cmd>Man<CR>', opts)
keymap('n', 'j', 'gj', opts)
keymap('n', 'k', 'gk', opts)
keymap('n', '<S-h>', '^', opts)
keymap('n', '<S-l>', '$', opts)
keymap('n', '<S-k>', '{', opts)
keymap('n', '<S-j>', '}', opts)
keymap('n', 'm', '%', opts)

-- File operations
keymap('n', '<leader>w', ':w<CR>', remap_opts)
keymap('n', '<leader>q', ':q<CR>', remap_opts)
keymap('n', '<leader>wq', ':wq<CR>', remap_opts)
keymap('n', '<esc><esc>', ':nohlsearch<CR><esc>', remap_opts)

-- Plugin keymaps (basic ones)
keymap('n', '<leader>n', ':Neotree filesystem reveal left<CR>', remap_opts)

-- DPP plugin manager setup
local dpp_cache = vim.fn.expand("~/.cache/nvim/dpp")

-- Plugin repository definitions
local plugins = {
  -- DPP core and extensions
  { repo = "Shougo/dpp.vim", essential = true },
  { repo = "vim-denops/denops.vim", essential = true },
  { repo = "Shougo/dpp-ext-installer" },
  { repo = "Shougo/dpp-ext-lazy" },
  { repo = "Shougo/dpp-protocol-git" },
  -- User plugins
  { repo = "folke/snacks.nvim", setup = function() require("snacks").setup() end },
  { repo = "numToStr/Comment.nvim", setup = function() require("Comment").setup() end },
  {
    repo = "hrsh7th/nvim-cmp",
    setup = function()
      local cmp = require("cmp")
      cmp.setup({
        mapping = cmp.mapping.preset.insert({
          ['<C-b>'] = cmp.mapping.scroll_docs(-4),
          ['<C-f>'] = cmp.mapping.scroll_docs(4),
          ['<C-Space>'] = cmp.mapping.complete(),
          ['<C-e>'] = cmp.mapping.abort(),
          ['<CR>'] = cmp.mapping.confirm({ select = true }),
        }),
        sources = cmp.config.sources({
          { name = 'nvim_lsp' },
          { name = 'copilot' },
        }, {
          { name = 'buffer' },
        })
      })
    end
  },
  { repo = "zbirenbaum/copilot-cmp", setup = function() require("copilot_cmp").setup() end },
  {
    repo = "zbirenbaum/copilot.lua",
    setup = function()
      require("copilot").setup({
        suggestion = { enabled = false },
        panel = { enabled = false },
      })
    end
  },
  {
    repo = "coder/claudecode.nvim",
    setup = function()
      require("claudecode").setup({
        terminal_cmd = "/opt/homebrew/bin/claude",
      })
    end
  },
  -- Neo-tree dependencies
  { repo = "nvim-lua/plenary.nvim" },
  { repo = "nvim-tree/nvim-web-devicons" },
  { repo = "MunifTanjim/nui.nvim" },
  {
    repo = "nvim-neo-tree/neo-tree.nvim",
    setup = function()
      require("neo-tree").setup({
        close_if_last_window = false,
        popup_border_style = "rounded",
        enable_git_status = true,
        enable_diagnostics = true,
        filesystem = {
          follow_current_file = {
            enabled = true,
          },
          use_libuv_file_watcher = true,
        },
      })
    end
  },
  {
    repo = "shaunsingh/nord.nvim",
    setup = function()
      require('nord').set()
    end
  },
  {
    repo = "obsidian-nvim/obsidian.nvim",
    setup = function()
      require("obsidian").setup({
        workspaces = {
          {
            name = "main",
            path = "/Users/ktaga/Obsidian/Main",
          },
        },

        -- Disable legacy commands
        legacy_commands = false,

        -- Optional, customize note creation
        note_id_func = function(title)
          -- Create note IDs in a Zettelkasten style with a timestamp and a suffix.
          local suffix = ""
          if title ~= nil then
            suffix = title:gsub(" ", "-"):gsub("[^A-Za-z0-9-]", ""):lower()
          else
            for _ = 1, 4 do
              suffix = suffix .. string.char(math.random(65, 90))
            end
          end
          return tostring(os.time()) .. "-" .. suffix
        end,

        -- Optional, customize note frontmatter
        note_frontmatter_func = function(note)
          return {
            id = note.id,
            aliases = note.aliases,
            tags = note.tags,
            created = os.date("%Y-%m-%d %H:%M"),
          }
        end,

        -- Optional, for daily notes
        daily_notes = {
          folder = "dailies",
          date_format = "%Y-%m-%d",
        },

        -- Optional, completion settings
        completion = {
          nvim_cmp = true,
          min_chars = 2,
        },
      })
    end
  },
  -- Treesitter dependency for render-markdown
  { repo = "nvim-treesitter/nvim-treesitter" },
  {
    repo = "MeanderingProgrammer/render-markdown.nvim",
    setup = function()
      require('render-markdown').setup({
        -- Toggle key
        enabled = true,
        -- Maximum file size for rendering (5MB)
        max_file_size = 5.0,
        -- Debounce rendering
        debounce = 100,
        -- Rendering presets
        preset = 'lazy',

        -- Heading configuration
        heading = {
          enabled = true,
          sign = true,
          icons = { '󰲡 ', '󰲣 ', '󰲥 ', '󰲧 ', '󰲩 ', '󰲫 ' },
        },

        -- Code block configuration
        code = {
          enabled = true,
          sign = true,
          style = 'full',
          position = 'left',
          language_pad = 0,
          left_pad = 0,
          right_pad = 0,
        },

        -- Checkbox configuration
        checkbox = {
          enabled = true,
          unchecked = {
            icon = '󰄱 ',
            highlight = 'RenderMarkdownUnchecked',
          },
          checked = {
            icon = '󰱒 ',
            highlight = 'RenderMarkdownChecked',
          },
        },

        -- Callout configuration
        callout = {
          note = { raw = '[!NOTE]', rendered = '󰋽 Note' },
          tip = { raw = '[!TIP]', rendered = '󰌶 Tip' },
          important = { raw = '[!IMPORTANT]', rendered = '󰅾 Important' },
          warning = { raw = '[!WARNING]', rendered = '󰀪 Warning' },
          caution = { raw = '[!CAUTION]', rendered = '󰳦 Caution' },
        },
      })
    end
  },
  {
    repo = "sindrets/diffview.nvim",
    setup = function()
      require('diffview').setup({
        diff_binaries = false,    -- Show diffs for binaries
        enhanced_diff_hl = false, -- See ':h diffview-config-enhanced_diff_hl'
        git_cmd = { "git" },      -- The git executable followed by default args.
        hg_cmd = { "hg" },        -- The hg executable followed by default args.
        use_icons = true,         -- Requires nvim-web-devicons
        show_help_hints = true,   -- Show hints for how to open the help panel
        watch_index = true,       -- Update views and listings on index changes
        icons = {                 -- Only applies when use_icons is true.
          folder_closed = "",
          folder_open = "",
        },
        signs = {
          fold_closed = "",
          fold_open = "",
          done = "✓",
        },
        view = {
          -- Configure the layout and behavior of different types of views.
          -- Available layouts:
          --  'diff1_plain'
          --    |'diff2_horizontal'
          --    |'diff2_vertical'
          --    |'diff3_horizontal'
          --    |'diff3_vertical'
          --    |'diff3_mixed'
          --    |'diff4_mixed'
          -- For more info, see ':h diffview-config-view.x.layout'.
          default = {
            -- Config for changed files, and staged files in diff views.
            layout = "diff2_horizontal",
            winbar_info = false,          -- See ':h diffview-config-view.x.winbar_info'
          },
          merge_tool = {
            -- Config for conflicted files in diff views during a merge or rebase.
            layout = "diff3_horizontal",
            disable_diagnostics = true,   -- Temporarily disable diagnostics for conflict buffers while in the view.
            winbar_info = true,           -- See ':h diffview-config-view.x.winbar_info'
          },
          file_history = {
            -- Config for changed files in file history views.
            layout = "diff2_horizontal",
            winbar_info = false,          -- See ':h diffview-config-view.x.winbar_info'
          },
        },
      })
    end
  },
  {
    repo = "nvim-lualine/lualine.nvim",
    setup = function()
      require('lualine').setup({
        options = {
          icons_enabled = true,
          theme = 'nord',
          component_separators = { left = "", right = ""},
          section_separators = { left = "", right = ""},
          disabled_filetypes = {
            statusline = {},
            winbar = {},
          },
          ignore_focus = {},
          always_divide_middle = true,
          globalstatus = true,
          refresh = {
            statusline = 1000,
            tabline = 1000,
            winbar = 1000,
          }
        },
        sections = {
          lualine_a = {'mode'},
          lualine_b = {'branch', 'diff', 'diagnostics'},
          lualine_c = {
            {
              'filename',
              path = 1, -- 0: Just the filename, 1: Relative path, 2: Absolute path, 3: Absolute path, with tilde as the home directory
              shorting_target = 40,
              symbols = {
                modified = '[+]',      -- Text to show when the file is modified.
                readonly = '[-]',      -- Text to show when the file is non-modifiable or readonly.
                unnamed = '[No Name]', -- Text to show for unnamed buffers.
              }
            }
          },
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
        extensions = {'neo-tree', 'quickfix'}
      })
    end
  },
  {
    repo = "nvim-telescope/telescope.nvim",
    setup = function()
      local telescope = require('telescope')
      local actions = require('telescope.actions')

      telescope.setup({
        defaults = {
          -- Default configuration for telescope goes here:
          mappings = {
            i = {
              ["<C-n>"] = actions.cycle_history_next,
              ["<C-p>"] = actions.cycle_history_prev,
              ["<C-j>"] = actions.move_selection_next,
              ["<C-k>"] = actions.move_selection_previous,
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
          file_sorter = require('telescope.sorters').get_fuzzy_file,
          file_ignore_patterns = { "node_modules", ".git/" },
          generic_sorter = require('telescope.sorters').get_generic_fuzzy_sorter,
          winblend = 0,
          border = {},
          borderchars = { "─", "│", "─", "│", "╭", "╮", "╯", "╰" },
          color_devicons = true,
          use_less = true,
          path_display = {},
          set_env = { ["COLORTERM"] = "truecolor" },
          file_previewer = require('telescope.previewers').vim_buffer_cat.new,
          grep_previewer = require('telescope.previewers').vim_buffer_vimgrep.new,
          qflist_previewer = require('telescope.previewers').vim_buffer_qflist.new,
        },
        pickers = {
          find_files = {
            theme = "dropdown",
          },
        },
        extensions = {}
      })
    end
  },
  {
    repo = "lewis6991/gitsigns.nvim",
    setup = function()
      require('gitsigns').setup({
        signs = {
          add          = { text = '┃' },
          change       = { text = '┃' },
          delete       = { text = '_' },
          topdelete    = { text = '‾' },
          changedelete = { text = '~' },
          untracked    = { text = '┆' },
        },
        signs_staged = {
          add          = { text = '┃' },
          change       = { text = '┃' },
          delete       = { text = '_' },
          topdelete    = { text = '‾' },
          changedelete = { text = '~' },
          untracked    = { text = '┆' },
        },
        signs_staged_enable = true,
        signcolumn = true,  -- Toggle with `:Gitsigns toggle_signs`
        numhl      = false, -- Toggle with `:Gitsigns toggle_numhl`
        linehl     = false, -- Toggle with `:Gitsigns toggle_linehl`
        word_diff  = false, -- Toggle with `:Gitsigns toggle_word_diff`
        watch_gitdir = {
          follow_files = true
        },
        auto_attach = true,
        attach_to_untracked = false,
        current_line_blame = false, -- Toggle with `:Gitsigns toggle_current_line_blame`
        current_line_blame_opts = {
          virt_text = true,
          virt_text_pos = 'eol', -- 'eol' | 'overlay' | 'right_align'
          delay = 1000,
          ignore_whitespace = false,
          virt_text_priority = 100,
        },
        current_line_blame_formatter = '<author>, <author_time:%R> - <summary>',
        sign_priority = 6,
        update_debounce = 100,
        status_formatter = nil, -- Use default
        max_file_length = 40000, -- Disable if file is longer than this (in lines)
        preview_config = {
          -- Options passed to nvim_open_win
          border = 'single',
          style = 'minimal',
          relative = 'cursor',
          row = 0,
          col = 1
        },
      })
    end
  },
  -- nvim-notify dependency for noice
  { repo = "rcarriga/nvim-notify" },
  {
    repo = "folke/noice.nvim",
    setup = function()
      require("noice").setup({
        lsp = {
          -- override markdown rendering so that **cmp** and other plugins use **Treesitter**
          override = {
            ["vim.lsp.util.convert_input_to_markdown_lines"] = true,
            ["vim.lsp.util.stylize_markdown"] = true,
            ["cmp.entry.get_documentation"] = true,
          },
        },
        -- you can enable a preset for easier configuration
        presets = {
          bottom_search = true, -- use a classic bottom cmdline for search
          command_palette = true, -- position the cmdline and popupmenu together
          long_message_to_split = true, -- long messages will be sent to a split
          inc_rename = false, -- enables an input dialog for inc-rename.nvim
          lsp_doc_border = false, -- add a border to hover docs and signature help
        },
        routes = {
          {
            filter = {
              event = "msg_show",
              any = {
                { find = "%d+L, %d+B" },
                { find = "; after #%d+" },
                { find = "; before #%d+" },
              },
            },
            view = "mini",
          },
        },
      })
    end
  },
  {
    repo = "windwp/nvim-autopairs",
    setup = function()
      require('nvim-autopairs').setup({
        check_ts = true, -- treesitter integration
        ts_config = {
          lua = {'string'},-- it will not add a pair on that treesitter node
          javascript = {'template_string'},
          java = false,-- don't check treesitter on java
        },
        disable_filetype = { "TelescopePrompt", "vim" },
        disable_in_macro = false, -- disable when recording or executing a macro
        disable_in_visualblock = false, -- disable when insert after visual block mode
        disable_in_replace_mode = true,
        ignored_next_char = [=[[%w%%%'%[%"%.%`%$]]=],
        enable_moveright = true,
        enable_afterquote = true, -- add bracket pairs after quote
        enable_check_bracket_line = true, -- check bracket in same line
        enable_bracket_in_quote = true, --
        enable_abbr = false, -- trigger abbreviation
        break_undo = true, -- switch for basic rule break undo sequence
        check_comma = true,
        map_cr = true,
        map_bs = true,  -- map the <BS> key
        map_c_h = false, -- Map the <C-h> key to delete a pair
        map_c_w = false, -- map <c-w> to delete a pair if possible
      })

      -- Integration with nvim-cmp
      local cmp_autopairs = require('nvim-autopairs.completion.cmp')
      local cmp = require('cmp')
      if cmp then
        cmp.event:on('confirm_done', cmp_autopairs.on_confirm_done())
      end
    end
  },
}

-- Initialize cache directory
if vim.fn.isdirectory(dpp_cache) == 0 then
  vim.fn.mkdir(dpp_cache, "p")
end

-- Helper function to get plugin path
local function get_plugin_path(repo)
  return dpp_cache .. "/repos/github.com/" .. repo
end

-- Clone a single plugin
local function clone_plugin(repo)
  local plugin_path = get_plugin_path(repo)
  if vim.fn.isdirectory(plugin_path) == 0 then
    local cmd = string.format("git clone --depth=1 https://github.com/%s %s", repo, plugin_path)
    local result = vim.fn.system(cmd)
    if vim.v.shell_error == 0 then
      print("✓ Cloned: " .. repo)
      return true
    else
      print("✗ Failed to clone: " .. repo)
      return false
    end
  else
    print("• Already exists: " .. repo)
    return true
  end
end

-- Load plugin into runtimepath
local function load_plugin(repo)
  local plugin_path = get_plugin_path(repo)
  if vim.fn.isdirectory(plugin_path) == 1 then
    vim.opt.runtimepath:prepend(plugin_path)
    return true
  end
  return false
end

-- Setup command to install all plugins
vim.api.nvim_create_user_command("DppSetup", function()
  print("Setting up dpp plugins...")
  local success_count = 0
  local total_count = #plugins

  for _, plugin in ipairs(plugins) do
    if clone_plugin(plugin.repo) then
      load_plugin(plugin.repo)
      success_count = success_count + 1
    end
  end

  print(string.format("Setup complete: %d/%d plugins ready. Restart Neovim.", success_count, total_count))
end, { desc = "Install and setup dpp plugins" })

-- Update command for existing plugins
vim.api.nvim_create_user_command("DppUpdate", function()
  print("Updating dpp plugins...")
  for _, plugin in ipairs(plugins) do
    local plugin_path = get_plugin_path(plugin.repo)
    if vim.fn.isdirectory(plugin_path) == 1 then
      local cmd = string.format("cd %s && git pull", plugin_path)
      vim.fn.system(cmd)
      if vim.v.shell_error == 0 then
        print("✓ Updated: " .. plugin.repo)
      else
        print("✗ Failed to update: " .. plugin.repo)
      end
    end
  end
  print("Update complete. Restart Neovim to apply changes.")
end, { desc = "Update all dpp plugins" })

-- Auto-load plugins if they exist
local essential_loaded = true
for _, plugin in ipairs(plugins) do
  if plugin.essential and not load_plugin(plugin.repo) then
    essential_loaded = false
    break
  elseif not plugin.essential then
    load_plugin(plugin.repo)
  end
end

-- Setup plugins with defer_fn for better startup performance
if essential_loaded then
  for _, plugin in ipairs(plugins) do
    if plugin.setup and load_plugin(plugin.repo) then
      vim.defer_fn(plugin.setup, 100)
    end
  end
else
  print("Essential dpp plugins not found. Run :DppSetup to install.")
end

-- Keymaps
vim.keymap.set('n', '<leader>ac', '<cmd>ClaudeCode<cr>', { desc = 'Toggle Claude Code' })
vim.keymap.set('n', '<leader>e', '<cmd>Neotree filesystem reveal left<cr>', { desc = 'Toggle Neo-tree' })
vim.keymap.set('n', '<leader>be', '<cmd>Neotree buffers reveal float<cr>', { desc = 'Neo-tree buffers' })

-- Obsidian keymaps (updated to new command format)
vim.keymap.set('n', '<leader>on', '<cmd>Obsidian new<cr>', { desc = 'New Obsidian note' })
vim.keymap.set('n', '<leader>oo', '<cmd>Obsidian open<cr>', { desc = 'Open note in Obsidian app' })
vim.keymap.set('n', '<leader>os', '<cmd>Obsidian search<cr>', { desc = 'Search Obsidian notes' })
vim.keymap.set('n', '<leader>ot', '<cmd>Obsidian today<cr>', { desc = 'Open today\'s note' })
vim.keymap.set('n', '<leader>ob', '<cmd>Obsidian backlinks<cr>', { desc = 'Show backlinks' })
vim.keymap.set('n', '<leader>ol', '<cmd>Obsidian links<cr>', { desc = 'Show links' })

-- Render-markdown keymaps
vim.keymap.set('n', '<leader>mr', '<cmd>RenderMarkdown toggle<cr>', { desc = 'Toggle markdown rendering' })
vim.keymap.set('n', '<leader>me', '<cmd>RenderMarkdown enable<cr>', { desc = 'Enable markdown rendering' })
vim.keymap.set('n', '<leader>md', '<cmd>RenderMarkdown disable<cr>', { desc = 'Disable markdown rendering' })

-- Diffview keymaps
vim.keymap.set('n', '<leader>dv', '<cmd>DiffviewOpen<cr>', { desc = 'Open diffview' })
vim.keymap.set('n', '<leader>dc', '<cmd>DiffviewClose<cr>', { desc = 'Close diffview' })
vim.keymap.set('n', '<leader>dh', '<cmd>DiffviewFileHistory<cr>', { desc = 'File history' })
vim.keymap.set('n', '<leader>df', '<cmd>DiffviewFileHistory %<cr>', { desc = 'Current file history' })
vim.keymap.set('n', '<leader>dr', '<cmd>DiffviewRefresh<cr>', { desc = 'Refresh diffview' })

-- Telescope keymaps
vim.keymap.set('n', '<leader>ff', '<cmd>Telescope find_files<cr>', { desc = 'Find files' })
vim.keymap.set('n', '<leader>fg', '<cmd>Telescope live_grep<cr>', { desc = 'Live grep' })
vim.keymap.set('n', '<leader>fb', '<cmd>Telescope buffers<cr>', { desc = 'Find buffers' })
vim.keymap.set('n', '<leader>fh', '<cmd>Telescope help_tags<cr>', { desc = 'Help tags' })
vim.keymap.set('n', '<leader>fr', '<cmd>Telescope oldfiles<cr>', { desc = 'Recent files' })
vim.keymap.set('n', '<leader>fc', '<cmd>Telescope commands<cr>', { desc = 'Commands' })
vim.keymap.set('n', '<leader>fk', '<cmd>Telescope keymaps<cr>', { desc = 'Keymaps' })
vim.keymap.set('n', '<leader>fs', '<cmd>Telescope current_buffer_fuzzy_find<cr>', { desc = 'Search in buffer' })

-- Git telescope
vim.keymap.set('n', '<leader>gc', '<cmd>Telescope git_commits<cr>', { desc = 'Git commits' })
vim.keymap.set('n', '<leader>gb', '<cmd>Telescope git_branches<cr>', { desc = 'Git branches' })
vim.keymap.set('n', '<leader>gs', '<cmd>Telescope git_status<cr>', { desc = 'Git status' })

-- Gitsigns keymaps
vim.keymap.set('n', ']c', '<cmd>Gitsigns next_hunk<cr>', { desc = 'Next git hunk' })
vim.keymap.set('n', '[c', '<cmd>Gitsigns prev_hunk<cr>', { desc = 'Previous git hunk' })
vim.keymap.set('n', '<leader>hs', '<cmd>Gitsigns stage_hunk<cr>', { desc = 'Stage hunk' })
vim.keymap.set('v', '<leader>hs', '<cmd>Gitsigns stage_hunk<cr>', { desc = 'Stage hunk' })
vim.keymap.set('n', '<leader>hr', '<cmd>Gitsigns reset_hunk<cr>', { desc = 'Reset hunk' })
vim.keymap.set('v', '<leader>hr', '<cmd>Gitsigns reset_hunk<cr>', { desc = 'Reset hunk' })
vim.keymap.set('n', '<leader>hS', '<cmd>Gitsigns stage_buffer<cr>', { desc = 'Stage buffer' })
vim.keymap.set('n', '<leader>hu', '<cmd>Gitsigns undo_stage_hunk<cr>', { desc = 'Undo stage hunk' })
vim.keymap.set('n', '<leader>hR', '<cmd>Gitsigns reset_buffer<cr>', { desc = 'Reset buffer' })
vim.keymap.set('n', '<leader>hp', '<cmd>Gitsigns preview_hunk<cr>', { desc = 'Preview hunk' })
vim.keymap.set('n', '<leader>hb', '<cmd>Gitsigns blame_line<cr>', { desc = 'Blame line' })
vim.keymap.set('n', '<leader>tb', '<cmd>Gitsigns toggle_current_line_blame<cr>', { desc = 'Toggle line blame' })
vim.keymap.set('n', '<leader>hd', '<cmd>Gitsigns diffthis<cr>', { desc = 'Diff this' })
vim.keymap.set('n', '<leader>hD', '<cmd>Gitsigns diffthis ~<cr>', { desc = 'Diff this ~' })
vim.keymap.set('n', '<leader>td', '<cmd>Gitsigns toggle_deleted<cr>', { desc = 'Toggle deleted' })

-- Text object
vim.keymap.set({'o', 'x'}, 'ih', ':<C-U>Gitsigns select_hunk<CR>', { desc = 'Select hunk' })

-- Clipboard integration with macOS
vim.opt.clipboard = "unnamedplus"

-- Copilot setup is handled by the plugin setup function above
