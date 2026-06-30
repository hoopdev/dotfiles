# dev CLI — 比較と拡張方針

`dev`（`home/mac/coder.nix`）を他の類似ツールと比較し、どこに投資し何をやらないかを
まとめた戦略メモ。使い方は [dev-tools.md](dev-tools.md) を参照。

## 1. dev の正体 — 4軸の交点

| 軸 | 内容 |
|---|---|
| ターミナルネイティブ | GUI を持たない。Zellij / SSH 越し / headless で動く |
| ローカル/リモート透過実行 | `dev run <name> "<cmd>"` がローカル cd か SSH かを解決して実行 |
| **エージェントから叩ける** | LLM がサブプロセスとして `dev run` を呼べる（人間 GUI 前提でない） |
| インフラ接着剤 | Coder + Cloudflare Access / 1Password SSH agent / Zellij / nu・pwsh / Nix |

**この4つを丸ごと埋める既製品は存在しない。** だから「競合」は単一でなく面ごとに分解される。

## 2. 他パッケージとの比較

### 2a. レイヤーマップ

| レイヤー | 代表ツール | dev との関係 |
|---|---|---|
| GUI オーケストレータ | **emdash**(YC W26) / Conductor / Claude Squad(TUI) / Crystal→Nimbalyst / vibe-kanban | 別レイヤー。人間が座る。worktree 分離・diff/PR/CI・kanban |
| 公式 | **Claude Code Agent View**(`claude agents`, v2.1.139+) / Agent Teams | claude の background session を内製管理。dev の隙間を一部吸収 |
| セッション/プロジェクト切替 | **sesh**(tmux/zellij/wezterm, zoxide+fzf) / tmux-sessionx / ghq+fzf | 思想が最も近い直接競合。ただしローカル限定 |
| リモート開発環境 | **DevPod**(loft, client-only, devcontainer) / Coder / Gitpod / Codespaces | DevPod は「環境をプロビジョニング」= Coder の代替であって dev の代替ではない |

`dev` はこれらの**下**を横断する「agent-callable な薄い接着層」。

### 2b. emdash 詳細（最も "やりたいこと" が近い）

emdash = **デスクトップ GUI**（Electron・ローカル SQLite）。worktree/branch 分離で複数エージェント並列、
SSH/SFTP リモート、diff/PR/CI レビュー、Linear/Jira 等の issue 取り込み、kanban。

**dev にあって emdash に無いもの**（= レイヤー差から来る強み）:

- エージェント自身が呼べる実行 API（`dev run` → stdout）
- ローカル/リモート透過の**任意コマンド**実行（emdash はタスク発注モデル）
- Coder + Cloudflare Access 経由 SSH / 1Password agent 多段転送
- nu・pwsh の Windows リモート / Zellij ネイティブ / `dev code`(Remote-SSH)
- `dev doctor` 診断 / `dev ps` が**外部起動**のエージェントも可視化
- ゼロインフラ（単一シェルスクリプト・DB/常駐なし・Nix 宣言・全マシン再現）
- worktree を強制しない in-place 実行

逆に emdash にあって dev に無い: **worktree-per-agent**・GUI ダッシュボード・diff/PR/CI・kanban。

### 2c. 競合は誰か

- **単一の競合製品は無い。**
- 思想面の直接competitor: **sesh**（ただしリモート実行も agent API も無く、核は無傷）
- リモート面の近接: **DevPod**（ただし Coder の代替）
- **真の脅威**: **Claude 公式 Agent View** と **Coder 本体**が機能を上に伸ばし、dev が埋めていた隙間を吸収すること

### 2d. `dev ps` 調査の結論（エージェント検出）

| ツール | バイナリ | `pgrep -x` | live 登録簿(PID/cwd/status) | dev の方針 |
|---|---|---|---|---|
| claude | node 上 | **macOS で壊れる** | **あり**(`~/.claude/sessions/*.json`, `claude agents --json`) | **JSON 採用**（status 付き、必須） |
| codex | native | 効く | なし（履歴 `~/.codex/sessions/**.jsonl` のみ） | pgrep 維持 |
| opencode | native | 効く | なし（`session list`=SQLite履歴 / `serve`=要サーバ） | pgrep 維持 |
| agy(1.0.13) | native | 効く | なし | pgrep 維持 |

claude だけが特殊（node ラッパー → pgrep 不可 → JSON 必須）。他3つは native なので pgrep が正解。

## 3. 実装済み（現状）

- **Tier 1①（SSH）**: ControlMaster 接続プーリング / `$SSH_AUTH_SOCK` 一本化（IdentityAgent ピン留め廃止、`ssh.nix` 方針と整合）/ `dev doctor` の bad-substitution バグ修正
- **Tier 1②（`dev ps`）**: claude を `claude agents --json` ベースに（macOS で壊れていた pgrep を修正、local+remote 両対応、status/kind 表示）。codex/opencode/agy は pgrep 維持。`/proc`→`lsof` フォールバックで macOS リモートも対応
- **Tier 2**: fzf `--preview`（picker に解決先の静的プレビュー、リモート SSH を張らず軽快）

### agent-callable フリート・オーケストレータ化（L0–L3 + TUI, 実装済み）

設計原理: **`dev … --json` を契約とし、LLM と TUI を同じ面の2クライアントにする。**

- **L0 — `--json` 基盤**: `dev targets` 新設、`ls/info/status/ps` に `--json`（既定の人間出力は不変、全 JSON は `jq -n` 生成）。`dev ps --json` が TUI の契約
- **L1 — fan-out**: `dev run --all` / `a,b,c` で多ターゲット並列実行、`--json` 結果。`run` のコマンド結合を `$*` に修正（引用符付きコマンドの既存バグも解消）
- **L2 — dispatch & 監督**: ラン台帳 `~/.dev/runs/`、`dev dispatch`（claude=`--bg` / codex・opencode=`nohup` デタッチ）、`attach`（Zellij タブ）/`logs`/`kill`（pgrep+cwd 照合）、**git worktree per agent**（兄弟 `.dev-worktrees/`）
- **L3 — 回収 & 通知**: `dev diff`（worktree-aware, numstat）/`dev pr`（push + `gh pr create`）/`dev notify`（Telegram, `~/.op-secrets`）/`dev watch`（waiting/finished を遷移検知して通知）
- **可視化**: `dev tui`（Rust ratatui TUI, `pkgs/dev-tui`, `ps --json` をライブ描画・waiting 最上段・enter/a/x/d で `dev` を shell out・review/model picker/batch dispatch）+ `dev dash`（fzf 簡易版）

## 4. 残りの方針

### 任意（未実装）

- **`dev do <project> <task>`** — プロジェクト内の just/mise タスク連携
- TUI の `d:dispatch` プロンプト強化、`watch` の常駐サービス化（launchd）

### やらない

- GUI / kanban → emdash の領分。bash/TUI で再実装しない（diff/pr は CLI で回収する方針に留める）
- `dev ls` の `column -t` 整形 → proxy 値 `coder-proxy %h` の空白で誤分割するため不可
- パイプ文字列 → Nix 宣言レジストリ → 秘匿ホスト名を git に出さない線引きの再設計が要り、リスク>リターン

### 監視対象

- **Claude Agent View / Agent Teams** と **Coder CLI** の進化 — dev の隙間（特に `dev ps`/`dev claude`/ルーティング）を吸収しうる。被ってきたら統合に寄せる
