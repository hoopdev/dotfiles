# Neovim プラグイン・スタック

このドキュメントは、dotfilesリポジトリで管理されているNeovimの設定とプラグイン構成を説明します。

## 概要

- **設定管理**: nixvim (Nix経由でNeovimを管理) + lazy.nvim (プラグイン管理)
- **カラースキーム**: Shonan (カスタムbase16テーマ、Stylix経由で自動適用)
- **Leaderキー**: `<Space>`
- **LocalLeaderキー**: `\`

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
| Stylix (base16 Shonan) | カラースキーム（Nix経由で自動注入） | - |
| [lualine.nvim](https://github.com/nvim-lualine/lualine.nvim) | ステータスライン | VeryLazy |
| [snacks.nvim](https://github.com/folke/snacks.nvim) | 多機能ユーティリティ (dashboard, notifier, zen mode, picker等) | ✗ |
| [markview.nvim](https://github.com/OXY2DEV/markview.nvim) | Markdownプレビュー・レンダリング | ✗ |

### ファイル操作・ナビゲーション

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [neo-tree.nvim](https://github.com/nvim-neo-tree/neo-tree.nvim) | ファイルエクスプローラー | コマンド/キーマップ |
| snacks.picker | ファジーファインダー（telescope.nvim代替） | snacks.nvim内蔵 |
| snacks.explorer | ファイルエクスプローラー | snacks.nvim内蔵 |

### コード編集

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [nvim-treesitter](https://github.com/nvim-treesitter/nvim-treesitter) | シンタックスハイライト・構文解析 | ✗ |
| [nvim-autopairs](https://github.com/windwp/nvim-autopairs) | 自動括弧補完 | InsertEnter |
| Neovim built-in (0.10+) | コメントトグル (gc/gcc) | - |

### LSP・補完

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig) | LSPクライアント設定 | BufReadPost/BufNewFile |
| [blink.cmp](https://github.com/saghen/blink.cmp) | Rust製自動補完エンジン（nvim-cmp代替） | InsertEnter |
| [friendly-snippets](https://github.com/rafamadriz/friendly-snippets) | スニペットコレクション | InsertEnter |
| [lazydev.nvim](https://github.com/folke/lazydev.nvim) | Lua LSP強化（Neovim設定用） | lua ft |

### AI・コーディング支援

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [copilot.lua](https://github.com/zbirenbaum/copilot.lua) | GitHub Copilot（ゴーストテキスト） | InsertEnter |
| [claudecode.nvim](https://github.com/coder/claudecode.nvim) | Claude Code統合 | コマンド/キーマップ |
| [opencode.nvim](https://github.com/NickvanDyke/opencode.nvim) | OpenCode AI統合 | config |

### Git統合

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [gitsigns.nvim](https://github.com/lewis6991/gitsigns.nvim) | Gitサイン・操作 | BufReadPost/BufNewFile |
| snacks.lazygit | Lazygit統合 | snacks.nvim内蔵 |
| snacks.git / snacks.gitbrowse | Git blame・ブラウズ | snacks.nvim内蔵 |

### Jupyter/ノートブック

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [molten-nvim](https://github.com/benlubas/molten-nvim) | Jupyter実行環境 | ipynb/markdown |
| [jupytext.nvim](https://github.com/GCBallesteros/jupytext.nvim) | ノートブック変換 | ipynb |
| [quarto-nvim](https://github.com/quarto-dev/quarto-nvim) | Quartoドキュメント | quarto/markdown |
| [otter.nvim](https://github.com/jmbuhr/otter.nvim) | 埋め込み言語サポート | quarto/markdown |

### ノート・ドキュメント

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [obsidian.nvim](https://github.com/obsidian-nvim/obsidian.nvim) | Obsidianノート統合 (Private/Work vaults) | markdown ft / コマンド |

### ターミナル統合

| プラグイン | 用途 | 遅延読み込み |
|-----------|------|-------------|
| [wezterm.nvim](https://github.com/willothy/wezterm.nvim) | WezTerm統合 | 条件付き (wezterm有無) |
| snacks.terminal | ターミナル切り替え | snacks.nvim内蔵 |

## 設定済みLSPサーバー

| サーバー | 言語 | 備考 |
|----------|------|------|
| pyright | Python | 型チェック専用 |
| ruff | Python | リンティング・フォーマット |
| ts_ls | TypeScript/JavaScript | |
| rust_analyzer | Rust | |
| gopls | Go | |
| lua_ls | Lua | lazydev.nvimによる強化 |

## 主要キーマップ

### 一般

| キー | 動作 |
|------|------|
| `<leader>w` | ファイル保存 |
| `<leader>q` | 終了 |
| `<leader>wq` | 保存して終了 |
| `<Esc><Esc>` | 検索ハイライト解除 |
| `j` / `k` | ビジュアル行移動 |
| `<S-h>` / `<S-l>` | 行頭/行末 |

### ファイル操作 (Neo-tree)

| キー | 動作 |
|------|------|
| `<leader>e` | ファイルエクスプローラー切り替え |
| `<leader>n` | 現在ファイルを表示 |
| `<leader>be` | バッファエクスプローラー |

### ファジーファインダー (Snacks Picker)

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
| `<leader>fS` | スマートピッカー |

### Git

| キー | 動作 |
|------|------|
| `<leader>gg` | Lazygit |
| `<leader>gb` | Git blame (行) |
| `<leader>gB` | Git browse |
| `<leader>gc` | Gitコミット |
| `<leader>gs` | Gitステータス |
| `<leader>gf` | 現ファイル履歴 |
| `<leader>gl` | Gitログ (cwd) |
| `<leader>gY` | リポジトリURL |

### Gitsigns

| キー | 動作 |
|------|------|
| `]c` | 次のhunk |
| `[c` | 前のhunk |
| `<leader>hs` | hunkをステージ |
| `<leader>hr` | hunkをリセット |
| `<leader>hp` | hunkプレビュー |
| `<leader>hb` | 行のblame |
| `<leader>hd` | diff表示 |
| `<leader>tb` | blame切り替え |
| `<leader>td` | deleted切り替え |

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
| `<leader>cf` | フォーマット |

### 補完 (blink.cmp)

| キー | 動作 |
|------|------|
| `<C-Space>` | 補完表示/ドキュメント表示 |
| `<CR>` | 確定 |
| `<Tab>` / `<S-Tab>` | 次/前の候補 |
| `<C-e>` | 閉じる |
| `<C-b>` / `<C-f>` | ドキュメントスクロール |

### Copilot

| キー | 動作 |
|------|------|
| `<M-l>` | 提案を受け入れ |
| `<M-w>` | 単語を受け入れ |
| `<M-j>` | 行を受け入れ |
| `<M-]>` / `<M-[>` | 次/前の提案 |

### Snacks.nvim

| キー | 動作 |
|------|------|
| `<leader>z` | Zenモード切り替え |
| `<leader>Z` | ズーム切り替え |
| `<leader>bd` | バッファ削除 |
| `<leader>bo` | 他バッファ削除 |
| `<leader>ba` | 全バッファ削除 |
| `<leader>N` | 通知履歴 |
| `<C-/>` | ターミナル切り替え |
| `<leader>se` | エクスプローラー |
| `<leader>cR` | ファイル名変更 |
| `]]` / `[[` | 次/前の参照 |

### Obsidian

| キー | 動作 |
|------|------|
| `<leader>on` | 新規ノート |
| `<leader>oo` | Obsidianで開く |
| `<leader>os` | ノート検索 |
| `<leader>oq` | クイック切替 |
| `<leader>ob` | バックリンク |
| `<leader>ol` | リンク表示 |
| `<leader>ot` | 今日のデイリーノート |
| `<leader>oy` | 昨日のデイリーノート |
| `<leader>om` | 明日のデイリーノート |
| `<leader>od` | デイリーノート一覧 |
| `<leader>ow` | ワークスペース切替 |

### Markview

| キー | 動作 |
|------|------|
| `<leader>mt` | Markview切り替え |
| `<leader>ms` | 分割プレビュー切り替え |

### AI統合

| キー | 動作 |
|------|------|
| `<leader>cc` | Claude Code起動 |
| `<C-.>` | OpenCode切り替え |
| `<C-a>` | OpenCodeに質問 |
| `<C-x>` | OpenCodeアクション選択 |
| `go` / `goo` | 範囲/行をOpenCodeに追加 |
| `+` / `-` | 数値インクリメント/デクリメント |

### Molten (Jupyter)

| キー | 動作 |
|------|------|
| `<leader>mi` | Molten初期化 |
| `<leader>e` | 評価 (operator) |
| `<leader>r` | 選択範囲を評価 |
| `<leader>rr` | セル再評価 |
| `<leader>oh` | 出力非表示 |
| `<leader>md` | セル削除 |
| `<leader>os` | 出力ウィンドウ表示 |

## パフォーマンス最適化

- **バイトコンパイル**: Luaファイルをバイトコンパイルして高速化
- **無効化されたビルトインプラグイン**: gzip, matchit, matchparen, netrwPlugin, tarPlugin, tohtml, tutor, zipPlugin
- **遅延読み込み**: ほとんどのプラグインは必要時にのみ読み込み
- **blink.cmp**: Rust製の高速補完エンジン

## Python依存パッケージ

Neovimプラグイン用のPythonパッケージ:
- `jupyter-client`
- `jupytext`
- `pynvim`

## 設定ファイル

- `home/common/cli/neovim.nix` - nixvim設定
- `home/common/cli/init.lua` - Lua設定（lazy.nvimプラグイン定義）
