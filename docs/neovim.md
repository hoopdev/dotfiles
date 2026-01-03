# Neovim プラグイン・スタック

このドキュメントは、dotfilesリポジトリで管理されているNeovimの設定とプラグイン構成を説明します。

## 概要

- **設定管理**: nixvim (Nix経由でNeovimを管理)
- **プラグインマネージャー**: lazy.nvim
- **カラースキーム**: Nord
- **Leaderキー**: `<Space>`

## プラグイン一覧

### コアプラグイン

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [lazy.nvim](https://github.com/folke/lazy.nvim) | プラグインマネージャー | - |
| [plenary.nvim](https://github.com/nvim-lua/plenary.nvim) | Luaライブラリ（依存関係） | ✓ |
| [nvim-web-devicons](https://github.com/nvim-tree/nvim-web-devicons) | ファイルアイコン | ✓ |
| [nui.nvim](https://github.com/MunifTanjim/nui.nvim) | UIコンポーネント | ✓ |

### UI・外観

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [nord.nvim](https://github.com/shaunsingh/nord.nvim) | Nordカラースキーム | ✗ (最優先) |
| [lualine.nvim](https://github.com/nvim-lualine/lualine.nvim) | ステータスライン | VeryLazy |
| [snacks.nvim](https://github.com/folke/snacks.nvim) | 多機能ユーティリティ (dashboard, notifier, zen mode等) | ✗ |

### ファイル操作・ナビゲーション

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [neo-tree.nvim](https://github.com/nvim-neo-tree/neo-tree.nvim) | ファイルエクスプローラー | コマンド/キーマップ |
| [telescope.nvim](https://github.com/nvim-telescope/telescope.nvim) | ファジーファインダー | キーマップ |

### コード編集

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [nvim-treesitter](https://github.com/nvim-treesitter/nvim-treesitter) | シンタックスハイライト・構文解析 | BufReadPost/BufNewFile |
| [nvim-autopairs](https://github.com/windwp/nvim-autopairs) | 自動括弧補完 | InsertEnter |
| [Comment.nvim](https://github.com/numToStr/Comment.nvim) | コメントトグル | VeryLazy |

### LSP・補完

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig) | LSPクライアント設定 | BufReadPost/BufNewFile |
| [nvim-cmp](https://github.com/hrsh7th/nvim-cmp) | 自動補完エンジン | InsertEnter |
| [cmp-nvim-lsp](https://github.com/hrsh7th/cmp-nvim-lsp) | LSP補完ソース | InsertEnter |
| [cmp-buffer](https://github.com/hrsh7th/cmp-buffer) | バッファ補完ソース | InsertEnter |
| [cmp-path](https://github.com/hrsh7th/cmp-path) | パス補完ソース | InsertEnter |

### AI・コーディング支援

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [copilot.lua](https://github.com/zbirenbaum/copilot.lua) | GitHub Copilot | InsertEnter |
| [copilot-cmp](https://github.com/zbirenbaum/copilot-cmp) | Copilot補完統合 | InsertEnter |
| [claudecode.nvim](https://github.com/coder/claudecode.nvim) | Claude Code統合 | コマンド/キーマップ |

### Git統合

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [gitsigns.nvim](https://github.com/lewis6991/gitsigns.nvim) | Gitサイン・操作 | BufReadPost/BufNewFile |

### Jupyter/ノートブック

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [molten-nvim](https://github.com/benlubas/molten-nvim) | Jupyter実行環境 | ipynb/markdown |
| [jupytext.nvim](https://github.com/GCBallesteros/jupytext.nvim) | ノートブック変換 | ipynb |
| [quarto-nvim](https://github.com/quarto-dev/quarto-nvim) | Quartoドキュメント | quarto/markdown |
| [otter.nvim](https://github.com/jmbuhr/otter.nvim) | 埋め込み言語サポート | quarto/markdown |

### ターミナル統合

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [wezterm.nvim](https://github.com/willothy/wezterm.nvim) | WezTerm統合 | - |

## 設定済みLSPサーバー

| サーバー | 言語 |
|----------|------|
| pyright | Python |
| ts_ls | TypeScript/JavaScript |
| rust_analyzer | Rust |
| gopls | Go |
| lua_ls | Lua |

## 主要キーマップ

### 一般

| キー | 動作 |
|------|------|
| `<leader>w` | ファイル保存 |
| `<leader>q` | 終了 |
| `<leader>wq` | 保存して終了 |
| `<Esc><Esc>` | 検索ハイライト解除 |

### ファイル操作 (Neo-tree)

| キー | 動作 |
|------|------|
| `<leader>e` | ファイルエクスプローラー切り替え |
| `<leader>n` | 現在ファイルを表示 |
| `<leader>be` | バッファエクスプローラー |

### ファジーファインダー (Telescope)

| キー | 動作 |
|------|------|
| `<leader>ff` | ファイル検索 |
| `<leader>fg` | テキスト検索 (grep) |
| `<leader>fb` | バッファ一覧 |
| `<leader>fh` | ヘルプタグ |
| `<leader>fr` | 最近のファイル |
| `<leader>fc` | コマンド一覧 |
| `<leader>fk` | キーマップ一覧 |
| `<leader>fs` | バッファ内検索 |

### Git (Telescope)

| キー | 動作 |
|------|------|
| `<leader>gc` | Gitコミット |
| `<leader>gb` | Gitブランチ |
| `<leader>gs` | Gitステータス |

### Git (Gitsigns)

| キー | 動作 |
|------|------|
| `]c` | 次のhunk |
| `[c` | 前のhunk |
| `<leader>hs` | hunkをステージ |
| `<leader>hr` | hunkをリセット |
| `<leader>hp` | hunkプレビュー |
| `<leader>hb` | 行のblame |
| `<leader>hd` | diff表示 |

### LSP

| キー | 動作 |
|------|------|
| `gd` | 定義へジャンプ |
| `gD` | 宣言へジャンプ |
| `gi` | 実装へジャンプ |
| `gr` | 参照一覧 |
| `K` | ホバー情報 |
| `<C-k>` | シグネチャヘルプ |
| `<leader>rn` | リネーム |
| `<leader>ca` | コードアクション |

### Snacks.nvim

| キー | 動作 |
|------|------|
| `<leader>z` | Zenモード切り替え |
| `<leader>Z` | ズーム切り替え |
| `<leader>gg` | Lazygit |
| `<leader>bd` | バッファ削除 |
| `<leader>N` | 通知履歴 |
| `<C-/>` | ターミナル切り替え |
| `<leader>se` | エクスプローラー |

### Snacks Picker

| キー | 動作 |
|------|------|
| `<leader>ps` | スマートピッカー |
| `<leader>pf` | ファイル検索 |
| `<leader>pg` | Grep |
| `<leader>pb` | バッファ |
| `<leader>pr` | 最近のファイル |

### Molten (Jupyter)

| キー | 動作 |
|------|------|
| `<leader>mi` | Molten初期化 |
| `<leader>e` | 評価 (operator) |
| `<leader>r` | 選択範囲を評価 |
| `<leader>rr` | セル再評価 |
| `<leader>oh` | 出力非表示 |

### Claude Code

| キー | 動作 |
|------|------|
| `<leader>cc` | Claude Code起動 |

## パフォーマンス最適化

- **バイトコンパイル**: Luaファイルをバイトコンパイルして高速化
- **無効化されたビルトインプラグイン**: gzip, matchit, matchparen, netrwPlugin, tarPlugin, tohtml, tutor, zipPlugin
- **遅延読み込み**: ほとんどのプラグインは必要時にのみ読み込み

## Python依存パッケージ

Neovimプラグイン用のPythonパッケージ:
- `jupyter-client`
- `jupytext`
- `pynvim`

## 設定ファイル

- `home/common/cli/neovim.nix` - nixvim設定
- `home/common/cli/init.lua` - Lua設定（lazy.nvimプラグイン定義）
