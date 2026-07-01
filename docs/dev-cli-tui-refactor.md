# dev CLI/TUI Refactor Plan

Date: 2026-07-02

## 背景

`dev` はすでに Rust workspace 化され、`pkgs/dev-core` / `pkgs/dev-cli` /
`pkgs/dev-tui` / `pkgs/dev-zellij` に分かれている。旧 bash からの移行計画は完了済みで、
今の課題は「移行」ではなく、CLI/TUI/Zellij が同じ概念を別々の経路で扱っていること。

現状のよい点:

- `dev-core` に config / git / agent / task store の共有実装がある。
- `dev-cli` は clap でコマンド面が型付けされている。
- `dev-tui` はデータ取得の一部を `dev-core` in-process に寄せている。
- `dev-zellij` は `dev snapshot --json` を読む単一経路になっており、境界が明確。

残っている主な問題:

- `dev task` が `dev agent` / `dev git` / `dev run` をサブプロセスで自己呼び出ししている。
- TUI の `App` が fleet / inbox / task board / usage / overlay / process tail を一つの巨大 state に持つ。
- TUI の input handler がキー入力処理と side effect 実行を同時に行っている。
- JSON を `serde_json::Value` として直接読む/書く箇所が多く、契約が型で保護されていない。
- `render.rs` が大きく、表示変更の blast radius が広い。

## 目標アーキテクチャ

```text
dev-core
  domain types
  task store
  agent registry / run registry
  git/status helpers
  snapshot builders
  task lifecycle service

dev-cli
  clap parsing
  human/json output formatting
  terminal handoff commands only where required

dev-tui
  state
  reducer: KeyEvent -> Action
  effects: Action -> dev-core call / dev command / pane operation
  render panels

dev-zellij
  dev snapshot --json consumer
  action commands via run_command
```

基本方針は `dev-core` をアプリケーション層にし、CLI/TUI/Zellij を薄い adapter にすること。
interactive attach、log follow、Zellij pane/tab 操作のように端末制御が本質の処理だけは
adapter 側に残す。

## Phase 0: 挙動固定

目的: リファクタ前に壊してはいけない契約をテストで固定する。

追加するテスト:

- `review_recommendation` の positive / negative precedence。
- task phase transitions: `draft -> planning -> needs_spec -> planned -> approved -> implementing -> review -> mergeable`。
- review/test artifact ID が task-local directory で採番されること。
- `dev run --all <cmd>` と multi-target failure propagation。
- `TaskDetail` / `BoardSnapshot` の serde roundtrip。

この段階では構造変更は最小限にし、既存 bugfix の再発防止を優先する。

## Phase 1: TaskService 抽出

目的: `dev task` の内部自己呼び出しを減らし、task lifecycle を `dev-core` へ移す。

新規候補:

```text
pkgs/dev-core/src/task_service.rs
pkgs/dev-core/src/task_store.rs
pkgs/dev-core/src/task_artifact.rs
```

責務:

- task lookup / context build / event append / phase update。
- plan / handoff / review / test artifact の保存。
- dispatch prompt の構築。
- review recommendation parsing。
- task summary update。

`dev-cli/src/cmd/task.rs` に残すもの:

- clap enum。
- usage error / process exit。
- human output / json output。
- interactive passthrough: attach, logs follow など。

優先して移す関数:

1. `cmd_harvest`
2. `cmd_test`
3. `cmd_review`
4. `cmd_dispatch` / `cmd_fix`
5. `cmd_pr`

自己呼び出しを残してよい例:

- `dev task attach` -> `dev agent attach`
- `dev task logs -f` -> live follow
- pager や Zellij pane を開く操作

それ以外の `dev agent dispatch` / `dev git diff` / `dev run` 相当は、可能な限り
`dev-core` の関数呼び出しにする。

## Phase 2: Snapshot Contract 統合

目的: CLI/TUI/Zellij の表示用データ契約を一本化する。

現状、Zellij は `dev snapshot --json` を読む構造で安定している。一方 TUI は
`dev-core` in-process call、`dev` subprocess、独自 model 変換が混在している。

提案:

```rust
pub struct FleetSnapshot {
    pub generated_at: String,
    pub targets: Vec<TargetSnapshot>,
    pub agents: Vec<AgentSnapshot>,
    pub git: Vec<GitSnapshot>,
    pub board: BoardSnapshot,
    pub usage: UsageSnapshot,
}
```

移行手順:

1. `dev-core` に snapshot builder を置く。
2. `dev snapshot --json` は builder の JSON 表示だけにする。
3. TUI worker は `FleetSnapshot` を in-process で取得する。
4. Zellij は引き続き `dev snapshot --json` を読む。

これで TUI と Zellij の task/inbox 表示差分を減らせる。

## Phase 3: TUI State 分割

目的: `App` の巨大 state を関心ごとに分ける。

候補:

```text
dev-tui/src/state/
  mod.rs
  fleet.rs
  tasks.rs
  inbox.rs
  usage.rs
  overlays.rs
  process.rs
```

分割例:

- `FleetState`: envs, git, groups, selection, filter, active_only, inspector view。
- `TaskBoardState`: tasks, questions, board column/row, task detail, selection。
- `InboxState`: selected question, answer buffer, answering flag。
- `UsageState`: claude/codex/agy usage and history。
- `OverlayState`: mode, result view, action menu, model picker。
- `ProcessState`: log tail pid, log lines, in-flight flags。

この段階では public fields を減らしすぎない。まずはファイルと所有境界を分け、
テスト可能な小さい methods を増やす。

## Phase 4: Input を Action/Effect 化

目的: キー入力処理から side effect を分離する。

現在は `input.rs` がキー入力を解釈しながら `Command::new("dev")` や pane 操作を実行する。
これを次の形に寄せる。

```rust
enum Action {
    MoveSelection(i32),
    Refresh,
    OpenActionMenu,
    Dispatch { target: String, tool: String, prompt: String },
    TaskApprove { id: String },
    TaskReview { id: String },
    Quit,
}

enum Effect {
    Request(Req),
    RunDev(Vec<String>),
    OpenPane { title: String, args: Vec<String> },
    Flash(String),
}
```

`handle_key` は `Action` を返し、`App::update(action)` が state mutation と effect を返す。
実際の subprocess / terminal operation は effect runner に閉じ込める。

利点:

- keymap の単体テストが可能。
- TUI 操作と CLI 契約のズレを検出しやすい。
- side effect failure を flash/error 表示へ集約できる。

## Phase 5: Render 分割

目的: 表示変更の blast radius を下げる。

候補:

```text
dev-tui/src/render/
  mod.rs
  theme.rs
  layout.rs
  fleet.rs
  inbox.rs
  tasks.rs
  usage.rs
  overlays.rs
```

順番:

1. theme/style helper を分離。
2. overlay rendering を分離。
3. fleet / inbox / tasks panel を分離。
4. layout calculation を分離。

この phase は挙動変更なしで進める。スクリーンショット差分を取れない環境でも、
compile と小さい pure helper tests で守れる形にする。

## Phase 6: JSON `Value` の封じ込め

目的: JSON 直接編集を `store` 層に閉じ込める。

追加候補:

```rust
pub struct TaskRecord {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub phase: TaskPhase,
    pub priority: Priority,
    pub assigned_tool: Option<String>,
    pub assigned_model: Option<String>,
    pub worktree_branch: Option<String>,
    pub worktree_path: Option<String>,
    pub validation: Validation,
    pub links: TaskLinks,
    pub summary: TaskSummary,
}
```

段階的にやる:

- 読み取り型を先に追加し、既存 JSON layout を変えない。
- 書き込み helper を型付き API にする。
- 最後に CLI/TUI から `serde_json::Value` 操作を消す。

## やらないこと

- DB 導入。現状の file store は `dev` のゼロ常駐/ゼロインフラ思想に合っている。
- TUI 全面書き換え。ratatui のまま、状態と effect 境界を整理する。
- async runtime 全面導入。まず同期 API の境界を整理し、必要な fan-out だけ並列化する。
- GUI/kanban 化。Zellij/TUI/CLI の範囲に留める。

## 完了条件

- `dev task` の通常フローで、自己 `dev` subprocess 呼び出しが interactive 系以外から消えている。
- TUI の key handling が side effect から分離され、主要 action が単体テストできる。
- `dev snapshot --json` と TUI が同じ snapshot builder を使っている。
- `render.rs` / `input.rs` / `app.rs` の巨大化が止まり、新規機能の追加先が明確。
- `serde_json::Value` は CLI output と store serialization の境界以外に露出しない。
