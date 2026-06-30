# dev CLI Rust 移行計画

現行の `home/mac/coder.nix` に 3000 行超の bash で実装された `dev` CLI を
Rust へ移行し、`notify`・`git2`・`tokio`・`zellij-tile` などのエコシステムを
使って拡張する計画。

## 現状の問題

| 問題 | 詳細 |
|---|---|
| Nix 文字列エスケープ | `''${VAR}` が至る所に必要、書き間違えると実行時クラッシュ |
| テスト不能 | 動作確認に `nh darwin switch` が必要 |
| 型安全なし | `jq: cannot index null` が実行時まで分からない |
| CLI/TUI 重複 | `DevTask` の JSON パースを bash 側と Rust 側で二重実装 |
| ポーリング | 30s 間隔の polling。エージェントの更新が即反映されない |
| Mac 専用 | Nix darwin 前提。kt-ubuntu でそのまま動かない |

## 目標アーキテクチャ

```
pkgs/
  dev-core/           # 共有ライブラリ (types, task store, git2, notify)
  dev-cli/            # CLI バイナリ (bash の代替)
  dev-tui/            # TUI (既存、dev-core に依存)
  dev-zellij/         # Zellij プラグイン (dev-core を WASM にコンパイル)

home/mac/coder.nix    # bash → 薄いシム + Nix パッケージ定義のみ
```

### 依存グラフ

```
dev-core
  ├── serde / serde_json   JSON 型
  ├── git2                 in-process git
  ├── notify               ファイル監視
  └── tokio (optional)     非同期ランタイム

dev-cli ──────── dev-core
dev-tui ──────── dev-core
dev-zellij ───── dev-core (wasm32-wasi)
```

## 主要ライブラリ

### `notify` — ファイル監視

```toml
notify = "6"
```

`~/.dev/projects/` の変更を inotify / kqueue でリアクティブに検知。
エージェントが `write-handoff` を書いた瞬間に TUI が更新される。
現状の 30s ポーリングが消える。

```rust
let (tx, rx) = std::sync::mpsc::channel();
let mut watcher = notify::recommended_watcher(tx)?;
watcher.watch(&task_store_path, RecursiveMode::Recursive)?;
// rx で変更イベントを受け取り TUI に Msg::DevTasks を送る
```

### `git2` — in-process git

```toml
git2 = "0.19"
```

`dev task diff / harvest / dev git status` がサブプロセスなしで動く。
worktree の作成・削除・ブランチ操作もすべて API 経由になり信頼性が上がる。

```rust
let repo = git2::Repository::open(&project_path)?;
let diff = repo.diff_index_to_workdir(None, None)?;
let stats = diff.stats()?;
// changed files を構造体として取得
```

### `tokio` — 非同期並列 SSH fan-out

```toml
tokio = { version = "1", features = ["full"] }
```

`dev status --all` が bash の `&` + `wait` ではなく真の async になる。
10 環境への SSH fan-out が最遅の 1 接続の時間で完了する。

```rust
let results = futures::future::join_all(
    envs.iter().map(|env| async move {
        ssh_run(env, "git status --short").await
    })
).await;
```

TUI のポーリングも全部 `tokio::spawn` で統一でき、`mpsc` チャネルはそのまま使える。

### `openssh` — SSH 接続管理

```toml
openssh = "0.10"
```

システムの `ssh` コマンドに依存しつつも Rust から接続プールを管理できる。
ControlMaster の代わりに Rust 側でセッションを保持し再利用できる。

### `zellij-tile` — Zellij ネイティブプラグイン

```toml
zellij-tile = "0.41"
```

**最重要拡張**。TUI を独立ウィンドウから Zellij のネイティブプラグインに昇格させる。
常時タスク状況をワークスペースに表示できる。

---

## Zellij 統合詳細

### プラグインの仕組み

Zellij プラグインは `wasm32-wasi` にコンパイルし、`~/.config/zellij/plugins/` に置く。
`ZellijPlugin` トレイトを実装する。

```rust
// dev-zellij/src/main.rs
use zellij_tile::prelude::*;

struct DevPlugin {
    tasks: Vec<DevTask>,
    questions: Vec<DevQuestion>,
    selected: usize,
    mode: PluginMode,
}

impl ZellijPlugin for DevPlugin {
    fn load(&mut self, _: BTreeMap<String, String>) {
        // ファイルシステム読み取り権限をリクエスト
        request_permission(&[PermissionType::ReadApplicationState,
                             PermissionType::RunCommands]);
        subscribe(&[EventType::Timer, EventType::Key]);
        set_timeout(5.0);  // 5 秒ごとに更新
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Timer(_) => {
                self.tasks = load_dev_tasks().0;
                self.questions = load_dev_tasks().1;
                true  // 再描画
            }
            Event::Key(key) => self.handle_key(key),
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        // dev-core の型を直接使ってレンダリング
        print_task_board(&self.tasks, &self.questions, rows, cols, self.selected);
    }
}
```

### 統合パターン 1: フローティングタスクボード

```kdl
// ~/.config/zellij/layouts/dev.kdl
layout {
    pane split_direction="vertical" {
        pane size="60%" { ... }  // 作業ペイン
        pane size="40%" {
            plugin location="file:~/.config/zellij/plugins/dev-zellij.wasm" {
                view "task-board"
            }
        }
    }
    pane size=1 borderless=true {
        plugin location="zellij:status-bar"
    }
}
```

### 統合パターン 2: ステータスバーへの埋め込み

Zellij のカスタムステータスバープラグインとして、blocking question 数と
現在 implementing 中のタスク数を常時表示する。

```
[tasks: 2 implementing | ⚠ 1 needs spec | 0 review]   [git: main Δ3]
```

### 統合パターン 3: `dev attach` をペイン分割で開く

タスクボードプラグインで `i` キーを押すと、Zellij の `run_command` アクション経由で
右ペインに `dev task attach <id>` を自動展開する。

```rust
run_command(&["dev", "task", "attach", &task_id], BTreeMap::new());
```

---

## 移行フェーズ

### Phase 0: Cargo ワークスペース設定（1 日）

```toml
# pkgs/Cargo.toml (新設)
[workspace]
members = ["dev-tui", "dev-core", "dev-cli", "dev-zellij"]
resolver = "2"
```

`dev-tui/src/task.rs` の `DevTask`・`DevQuestion`・`load_dev_tasks()` を
`dev-core` に移動し、`dev-tui` は `dev-core` に依存する形にする。

**成果**: 既存 TUI の動作は変わらず、共有基盤が整う。

### Phase 1: dev-core — タスクストア（3 日）

`dev task *` の CLI ロジックを `dev-core` + `dev-cli` に実装。

```
dev-core/src/
  lib.rs
  task/
    mod.rs       # DevTask, DevQuestion, TaskDetail 型
    store.rs     # load / save / update 操作
    events.rs    # イベントログ append
    questions.rs # question CRUD
  git.rs         # git2 ラッパー
  watch.rs       # notify ラッパー

dev-cli/src/
  main.rs
  cmd/
    task.rs      # dev task * サブコマンド
    git.rs       # dev git * サブコマンド
    agent.rs     # dev agent * (一部 bash に委譲)
```

bash 側では `dev task "$@"` → `${devCli}/bin/dev-task "$@"` に差し替え。
Phase 1-5 で実装した bash の `task)` case が消える。

**成果**: タスクストア層が型安全になり、`cargo test` で単体テスト可能になる。

### Phase 2: git2 統合（2 日）

`dev task diff`・`harvest`・`dev git status/diff` を git2 経由に切り替え。

```rust
pub fn harvest(task: &mut DevTask, project_path: &Path) -> Result<HarvestResult> {
    let repo = git2::Repository::open(project_path)?;
    let worktree = repo.find_worktree(&task.worktree_branch)?;
    let wt_repo = worktree.open()?;
    let diff = wt_repo.diff_head_to_index(None, None)?;
    let files: Vec<String> = diff.deltas()
        .map(|d| d.new_file().path().unwrap().to_string_lossy().into())
        .collect();
    task.summary.diff_files = files;
    Ok(HarvestResult { ... })
}
```

**成果**: サブプロセス起動コスト消滅。worktree 操作の信頼性向上。

### Phase 3: tokio + notify — リアクティブ更新（2 日）

TUI のポーリングを notify ベースのウォッチャーに切り替え。

```rust
// dev-tui/src/watcher.rs
pub async fn watch_task_store(tx: mpsc::Sender<Msg>) -> notify::Result<()> {
    let (notify_tx, notify_rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(notify_tx)?;
    watcher.watch(&task_store_path(), RecursiveMode::Recursive)?;
    for event in notify_rx {
        if event?.kind.is_modify() || event?.kind.is_create() {
            let (tasks, questions) = load_dev_tasks();
            let _ = tx.send(Msg::DevTasks(tasks, questions));
        }
    }
    Ok(())
}
```

**成果**: エージェントが handoff を書いた瞬間に TUI が更新される。ポーリング完全撤廃。

### Phase 4: dev-agent — SSH 並列化（3 日）

`dev agent ps --json` の SSH fan-out を tokio で並列化。

```rust
pub async fn ps_all(envs: &[Env]) -> Vec<AgentStatus> {
    futures::future::join_all(
        envs.iter().map(|env| ps_remote(env))
    ).await.into_iter().flatten().collect()
}
```

`dev run --all` も同様。全環境への fan-out が体感 3–5× 高速化。

**成果**: `dev tui` 起動直後の待ち時間がほぼゼロになる。

### Phase 5: dev-zellij プラグイン（3 日）

```
dev-zellij/
  Cargo.toml    # target = ["cdylib"], target wasm32-wasi
  src/
    main.rs     # ZellijPlugin 実装
    render.rs   # タスクボード・Inbox を Zellij 座標系で描画
    keys.rs     # キーバインド (dev-core の型を直接操作)
```

**重要**: `dev-core` は I/O を除けば `wasm32-wasi` でコンパイル可能。
git2 は WASM 非対応のため、プラグイン側では `std::fs` で直接 JSON を読む。

```toml
# dev-zellij/Cargo.toml
[lib]
crate-type = ["cdylib"]

[dependencies]
dev-core = { path = "../dev-core", default-features = false, features = ["wasm"] }
zellij-tile = "0.41"
serde_json = "1"
```

```nix
# pkgs/dev-zellij/default.nix
pkgs.rustPlatform.buildRustPackage {
  CARGO_BUILD_TARGET = "wasm32-wasi";
  installPhase = ''
    install -Dm644 target/wasm32-wasi/release/dev_zellij.wasm \
      $out/share/zellij/plugins/dev-zellij.wasm
  '';
}
```

**成果**: タスクボードが Zellij 常駐ペインになる。`dev tui` を別途開く必要がなくなる。

---

## coder.nix の最終形

移行完了後の `coder.nix` は大幅に薄くなる。

```nix
let
  devCore = pkgs.rustPlatform.buildRustPackage { ... };   # dev-core
  devCli  = pkgs.rustPlatform.buildRustPackage { ... };   # dev-cli
  devTui  = pkgs.rustPlatform.buildRustPackage { ... };   # dev-tui (既存)
  devZellij = pkgs.rustPlatform.buildRustPackage { ... }; # dev-zellij

  # bash に残るもの: SSH ラッパー、fzf UI、coder-proxy など (~200 行)
  devCmd = pkgs.writeShellScriptBin "dev" ''
    case "$1" in
      task)    shift; exec ${devCli}/bin/dev-task "$@" ;;
      tui)     exec ${devTui}/bin/dev-tui "$@" ;;
      agent)   shift; _dev_agent "$@" ;;  # SSH 操作は bash 残存
      git)     shift; exec ${devCli}/bin/dev-git "$@" ;;
      run|shell|code|ls|info|status|doctor|targets)
               exec ${devCli}/bin/dev "$1" "${@:2}" ;;
      *)       exec ${devCli}/bin/dev "$@" ;;
    esac
  '';
in { home.packages = [ devCmd devTui devZellij ... ]; }
```

bash に残るのは SSH セッション管理、fzf 対話 UI、coder-proxy など約 200 行のみ。

---

## 移行タイムライン

| フェーズ | 内容 | 期間 | 優先度 |
|---|---|---|---|
| 0 | Cargo ワークスペース + dev-core 骨格 | 1 日 | 必須 |
| 1 | タスクストア CLI (dev task *) | 3 日 | 高 |
| 2 | git2 統合 | 2 日 | 中 |
| 3 | notify リアクティブ更新 | 2 日 | 高 |
| 4 | tokio SSH 並列化 | 3 日 | 中 |
| 5 | Zellij プラグイン | 3 日 | **最重要** |

合計約 2 週間。フェーズ 0→1→3→5 の順で進めれば Zellij 統合まで最短経路で到達できる。

---

## 移行しない部分

以下は bash のまま残す（移行コストに対してメリットが薄い）:

- `dev shell <target>` — `exec ssh ...` の 1 行
- `dev code <target>` — `exec code ...` の 1 行
- `coder-proxy` — Coder 専用の CloudFlare ラッパー
- `dev notify` — Telegram curl 呼び出し
- fzf 対話 UI (`dev shell`・`dev claude` の選択 UI)

---

## 参考: Zellij プラグイン設定例

```kdl
// ~/.config/zellij/layouts/dev-workspace.kdl
layout {
    pane_template name="task-sidebar" {
        plugin location="file:~/.nix-profile/share/zellij/plugins/dev-zellij.wasm"
    }
    pane split_direction="vertical" {
        pane             // メイン作業ペイン
        task-sidebar size=40
    }
    pane size=2 borderless=true {
        plugin location="file:~/.nix-profile/share/zellij/plugins/dev-zellij.wasm" {
            view "statusbar"   // タスク数 + blocking 数のみ表示
        }
    }
}
```
