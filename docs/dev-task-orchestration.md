# dev Task Orchestration 仕様

Status: implemented baseline + design reference. 現行のリファクタ計画は
[dev-cli-tui-refactor.md](dev-cli-tui-refactor.md) を参照。

`dev tui` を agent の稼働監視から、agent 並列開発のための Task / Plan / Question
管制塔へ拡張する。目的は「多数の agent を起動すること」ではなく、曖昧な仕様を減らし、
作業範囲を分離し、実装成果を安全に回収すること。

## 原則

1. **Task が主語**
   - agent process は task の実行手段であり、主オブジェクトではない。
   - TUI の主画面は agent list ではなく Task Board とする。

2. **Plan 承認前は原則編集禁止**
   - agent はまず調査、質問、実装案、検証方針を出す。
   - 不明点がある場合は `dev task ask` で人間に確認し、そこで停止する。
   - 人間が `approve` するまで implementation phase に進めない。

3. **共有状態は repo 外に置く**
   - worktree ごとに分岐してはいけない情報は `~/.dev/projects/` に置く。
   - worktree 内には成果物と git diff だけを残す。

4. **すべて JSON 契約を持つ**
   - TUI と外部 LLM orchestration は同じ `dev task ... --json` を読む。
   - 人間向け出力は便利にしてよいが、機械契約は安定させる。

5. **review は task lifecycle に統合する**
   - `dev agent review` は低レベル primitive として残す。
   - TUI と上位CLIでは `dev task review <task-id>` を使い、結果を task artifact に保存する。

## ストレージ

共有 control plane はローカルマシンの `$HOME/.dev/projects/` に置く。
remote project の task も、基本は起点端末側に保存する。remote 側には実行に必要な
brief/context を都度渡す。

```text
~/.dev/projects/
  index.json
  <project-id>/
    project.json
    project.md
    plan.md
    decisions.jsonl
    questions.jsonl
    tasks/
      <task-id>/
        task.json
        brief.md
        plan.md
        approved-plan.md
        context.md
        events.jsonl
        handoff.md
        reviews/
          <review-id>.json
          <review-id>.md
        test-results/
          <run-id>.json
          <run-id>.log
```

### project-id

`project-id` は `dev targets --json` の target name を基本にする。rename に備え、
`project.json` に resolved path と remote env を保存する。

```json
{
  "id": "graph-fem-server",
  "target": "graph-fem-server",
  "location": "remote",
  "env": "bf-e",
  "path": "/home/ktaga/src/graph-fem",
  "created_at": "2026-06-30T00:00:00Z",
  "updated_at": "2026-06-30T00:00:00Z"
}
```

## データモデル

### ID

ID は人間が TUI で読みやすく、ファイル名として安全な形式にする。

```text
Task:     T-YYYYMMDD-NNN
Question: Q-YYYYMMDD-NNN
Review:   R-YYYYMMDD-NNN
Test run: V-YYYYMMDD-NNN
```

採番は project ごとに日次連番とする。衝突した場合は次の番号を使う。

### Task

`task.json` は task の正規状態を持つ。本文や長い plan は markdown に分ける。

```json
{
  "id": "T-20260630-001",
  "project_id": "graph-fem-server",
  "title": "virtual DAC offset propagation bugfix",
  "phase": "draft",
  "priority": "normal",
  "created_at": "2026-06-30T00:00:00Z",
  "updated_at": "2026-06-30T00:00:00Z",
  "created_by": "human",
  "assigned_tool": null,
  "assigned_model": null,
  "worktree_branch": null,
  "worktree_path": null,
  "scope": {
    "paths": [],
    "allowed_paths": [],
    "forbidden_paths": [],
    "risk": "unknown"
  },
  "validation": {
    "commands": [],
    "required": true
  },
  "links": {
    "run_id": null,
    "session_id": null,
    "pr_url": null
  },
  "summary": {
    "latest_question": null,
    "latest_handoff": null,
    "diff_files": [],
    "review_status": "none",
    "test_status": "unknown"
  }
}
```

### Task Phase

```text
draft
  人間またはagentが作った未整理の依頼。

planning
  agent が repo を読み、質問と plan を作る。コード編集禁止。

needs_spec
  agent が仕様確認を要求して停止している。

planned
  plan は出たが、人間がまだ承認していない。

approved
  人間が plan を承認した。実装開始可能。

implementing
  agent が approved-plan に従って実装中。

review
  実装完了後、review / diff / test の回収待ち。

needs_fix
  review または test により修正が必要。

mergeable
  差分、review、validation が揃い、人間が統合可能と判断できる。

merged
  PR 作成または merge 完了。

rejected
  採用しない。worktree cleanup 候補。

killed
  実行中 agent を停止した。
```

許可する主な遷移:

```text
draft -> planning
planning -> needs_spec
planning -> planned
needs_spec -> planning
planned -> approved
approved -> implementing
implementing -> needs_spec
implementing -> review
review -> needs_fix
review -> mergeable
needs_fix -> approved
mergeable -> merged
* -> rejected
* -> killed
```

### Question

agent が人間に仕様確認するための正規インターフェイス。

`questions.jsonl` と task 配下 `events.jsonl` の両方に記録する。

```json
{
  "id": "Q-20260630-001",
  "task_id": "T-20260630-001",
  "project_id": "graph-fem-server",
  "status": "open",
  "severity": "blocking",
  "category": "behavior",
  "question": "offset propagation should apply before or after virtual gate matrix multiplication?",
  "options": [
    {
      "id": "A",
      "label": "before matrix multiplication",
      "impact": "keeps physical gate offsets independent from virtual transforms"
    },
    {
      "id": "B",
      "label": "after matrix multiplication",
      "impact": "matches current observed behavior but changes interpretation"
    }
  ],
  "agent_recommendation": "A",
  "context": "Current tests imply A, implementation appears closer to B.",
  "created_at": "2026-06-30T00:00:00Z",
  "answered_at": null,
  "answer": null
}
```

`severity`:

- `blocking`: 回答まで実装禁止。
- `nonblocking`: agent は保守的な仮定で plan を出せる。
- `note`: 人間に共有するだけ。

`category`:

- `behavior`
- `scope`
- `compatibility`
- `api`
- `ux`
- `test`
- `migration`
- `security`
- `release`

### Plan

`plan.md` は agent が作る案。`approved-plan.md` は人間承認済みの固定版。

必須セクション:

```markdown
# Plan

## Understanding

## Open Questions

## Proposed Behavior

## Scope

## Files To Touch

## Files Not To Touch

## Implementation Steps

## Validation

## Risks

## Rollback
```

`Open Questions` が blocking を含む場合、phase は `needs_spec` になる。
blocking question がない場合、phase は `planned` になる。

### Event

`events.jsonl` は append-only。TUI はこれを timeline として表示できる。

```json
{
  "ts": "2026-06-30T00:00:00Z",
  "type": "phase_changed",
  "actor": "dev",
  "from": "planning",
  "to": "needs_spec",
  "message": "blocking question opened"
}
```

代表的な `type`:

- `task_created`
- `phase_changed`
- `agent_dispatched`
- `question_opened`
- `question_answered`
- `plan_written`
- `plan_approved`
- `implementation_started`
- `handoff_written`
- `review_completed`
- `test_completed`
- `diff_harvested`
- `pr_created`
- `task_rejected`

## CLI

### 作成と閲覧

```bash
dev task new <project> --title <title> [--brief <text>] [--json]
dev task list [<project>] [--phase <phase>] [--json]
dev task show <task-id> [--json]
dev task context <task-id> [--json|--markdown]
dev task events <task-id> [--json]
```

`context` は agent に渡す共有文脈を生成する。最低限以下を含む。

- project.md
- project plan.md
- task brief.md
- task plan.md または approved-plan.md
- decisions
- open questions
- scope
- validation commands

### Planning

```bash
dev task plan <task-id> [--tool claude|codex|opencode|agy] [--model <model>] [--json]
dev task ask <task-id> <question> [--category <c>] [--severity <s>] [--json]
dev task answer <question-id> <answer> [--json]
dev task write-plan <task-id> [--file <path>] [--json]
dev task approve <task-id> [--json]
dev task reject <task-id> [--reason <text>] [--json]
```

`dev task plan` は planning prompt で background agent を起動する。
この phase の agent には「コード編集禁止」を明示する。

`dev task approve` は現在の `plan.md` を `approved-plan.md` にコピーし、phase を
`approved` にする。blocking open question がある場合は失敗する。

`dev task write-plan` は agent が planning phase の成果を保存するためのコマンド。
標準入力または `--file` の内容を `tasks/<task-id>/plan.md` に書き、blocking open
question がなければ phase を `planned` にする。

### 実装

```bash
dev task dispatch <task-id> [--tool <tool>] [--model <model>] [--worktree <branch>] [--json]
dev task attach <task-id>
dev task logs <task-id> [-f] [--json]
dev task kill <task-id> [--json]
dev task write-handoff <task-id> [--file <path>] [--json]
dev task handoff <task-id> [--json|--markdown]
```

`dispatch` は `approved` または `needs_fix` の task のみ受ける。
worktree branch の既定値は `task/<task-id-slug>`。

`dev task write-handoff` は agent が実装終了時または停止前に実行する。
標準入力または `--file` の内容を `handoff.md` に書き、phase を `review` にする。
ただし blocking question が新しく開かれている場合は `needs_spec` を優先する。

### 回収

```bash
dev task harvest <task-id> [--json]
dev task diff <task-id> [--stat] [--json]
dev task test <task-id> [--cmd <cmd>] [--json]
dev task review <task-id> [--tool <tool>] [--json]
dev task fix <task-id> [--tool <tool>] [--json]
dev task pr <task-id> [--title <title>] [--base <base>] [--draft] [--json]
```

`harvest` は worktree と run meta から以下を更新する。

- changed files
- diff stat
- current branch
- latest handoff
- latest run status
- phase 推定

`review` は `dev agent review` を呼び、結果を `reviews/` に保存する。
review が finding を返した場合は `needs_fix`、問題なしなら validation 状態に応じて
`mergeable` に進める。

### JSON 契約

すべての `--json` は最低限次を返す。

```json
{
  "ok": true,
  "task_id": "T-20260630-001",
  "project_id": "graph-fem-server",
  "phase": "planned",
  "message": "plan written"
}
```

失敗時:

```json
{
  "ok": false,
  "error": "blocking_questions_open",
  "message": "cannot approve while blocking questions are open",
  "task_id": "T-20260630-001"
}
```

## TUI

### 主画面

`dev tui` の既定ビューを Task Board にする。agent monitor は `p` または `agents` tab に移す。

```text
Needs Spec | Planned | Running | Review | Needs Fix | Mergeable
```

各 lane は task card を表示する。

```text
T-001  virtual DAC offset propagation
       graph-fem-server  planned  scope: src/core/*
       q:0  diff:0  tests:unknown  review:none
```

### 右ペイン

選択 task の詳細:

- title / phase / project
- brief
- approved plan または current plan
- open questions
- scope
- assigned tool/model
- worktree branch/path
- changed files
- validation commands
- latest handoff
- review summary
- test summary

### キー

```text
n  new task
p  planning agent
?  open questions / answer selected question
A  approve plan
i  implement / dispatch
a  attach
l  logs
h  harvest
d  diff
t  test
r  review
f  fix
m  pr / merge action
x  kill / reject
tab switch Task Board / Agents / Inbox / Usage
```

### Needs Spec Inbox

open blocking questions は task lane と別に inbox としても表示する。
TUI 起動時に blocking question があれば最上段に出す。

回答UIは以下を表示する。

- question
- agent recommendation
- options
- impact
- freeform answer

回答後は event を追加し、task phase を `planning` に戻す。

## Gate

CLI と TUI は以下の gate を守る。agent prompt だけに依存しない。

| Gate | 条件 | 失敗時 |
|---|---|---|
| `approve` | blocking open question がない、`plan.md` が存在する | `blocking_questions_open` / `plan_missing` |
| `dispatch` | phase が `approved` または `needs_fix`、`approved-plan.md` が存在する | `task_not_approved` |
| `review` | worktree または diff が存在する | `diff_missing` |
| `mergeable` | review が pass、required validation が pass または human override | `validation_missing` |
| `pr` | phase が `mergeable` | `task_not_mergeable` |

`--force` は `dispatch` の conflict warning と validation override だけに使える。
blocking question と approval gate は `--force` でも越えない。

## Agent Prompt 契約

### Planning Prompt

`dev task plan` が agent に渡す prompt の骨子:

```text
You are planning dev task <task-id> for project <project-id>.

Rules:
- Do not edit files.
- Inspect the repository and existing tests.
- Read the shared task context from `dev task context <task-id> --markdown`.
- If behavior, scope, compatibility, API, UX, migration, release, or validation is ambiguous,
  run `dev task ask <task-id> "<question>" --category <category> --severity blocking`
  and stop.
- If there are no blocking questions, write a plan using:
  dev task write-plan <task-id>
- The plan must include understanding, proposed behavior, files to touch,
  files not to touch, implementation steps, validation, risks, and rollback.
- Do not implement until the task is approved.
```

agent は plan を標準出力だけで返さず、`dev task write-plan <task-id>` を使って
共有 store に保存する。保存に失敗した場合は実装へ進まず、失敗理由を handoff として残す。

### Implementation Prompt

`dev task dispatch` が agent に渡す prompt の骨子:

```text
You are implementing approved dev task <task-id>.

Rules:
- Read `dev task context <task-id> --markdown`.
- Implement only the approved plan.
- Do not broaden scope.
- Respect allowed_paths and forbidden_paths.
- If the approved plan is insufficient or contradicts the code, run
  `dev task ask <task-id> "<question>" --category <category> --severity blocking`
  and stop.
- Run the declared validation commands when feasible.
- At the end, write a handoff with changed files, tests run, results, risks,
  and follow-up.
```

### Review Prompt

`dev task review` の prompt:

```text
Review dev task <task-id>.

Check:
- The diff implements the approved plan.
- The diff does not touch forbidden scope.
- Tests are sufficient or missing tests are clearly reported.
- There are no obvious regressions, security issues, or behavioral surprises.

Return:
- findings ordered by severity
- missing validation
- merge recommendation: mergeable / needs_fix / reject
```

## Conflict Detection

初期実装では lightweight な scope overlap のみ行う。

`task.scope.allowed_paths` または harvested `diff_files` が重なる task を検出する。

```text
conflict: exact file overlap
warning: same top-level directory
none: no known overlap
```

TUI は conflict を `!` 表示する。`dispatch` 時に conflict がある場合は警告するが、
`--force` で進められる。

## 既存機能との対応

| 既存 | 新しい位置づけ |
|---|---|
| `dev agent dispatch` | 低レベル primitive。`dev task dispatch` から呼ぶ |
| `dev agent ps` | agent tab と task の実行状態補完に使う |
| `dev agent logs` | `dev task logs` から task の run meta を解決して呼ぶ |
| `dev agent review` | 低レベル primitive。`dev task review` が結果を保存する |
| `dev git diff` | `dev task diff` / `harvest` が利用する |
| `dev git pr` | `dev task pr` が利用する |
| `~/.dev/tui-tasks.jsonl` | 移行対象。新規は `~/.dev/projects/*/tasks/*` |

## 実装段階

以下は初期実装時の段階メモ。Phase 1-5 の基礎機能は Rust workspace
（`pkgs/dev-core`, `pkgs/dev-cli`, `pkgs/dev-tui`, `pkgs/dev-zellij`）に移行済み。
今後の焦点は `dev task` lifecycle を `dev-core` の service API に寄せ、
CLI/TUI/Zellij の snapshot 契約を統一すること。

### Phase 1: Task Store と CLI

- `dev task new/list/show/context/events --json`
- `dev task ask/answer`
- `dev task approve/reject`
- local filesystem store
- 既存 TUI task history は読むだけ維持

### Phase 2: Planning Loop

- `dev task plan`
- planning prompt
- `needs_spec` phase
- TUI Needs Spec Inbox
- `planned -> approved`

### Phase 3: Implementation Loop

- `dev task dispatch`
- worktree branch 自動命名
- existing `dev agent dispatch` 連携
- `handoff.md`
- `harvest`

### Phase 4: Review 統合

- `dev task review`
- review artifact 保存
- phase 自動遷移
- `needs_fix -> approved -> implementing`

### Phase 5: Integration Queue

- conflict detection
- test result tracking
- mergeable lane
- `dev task pr`
- rejected/killed cleanup support

## 未決定事項

1. task lifecycle service の境界。
   - task store 自体は Rust 実装済み。
   - 次は `dev task` 内の自己 subprocess 呼び出しを `dev-core` API に置き換える。

2. remote project の task store を local に固定するか、remote に mirror するか。
   - 初期は local 固定。
   - remote agent には `dev task context` の生成物を prompt として渡す。

3. planning agent が `write-plan` せずに通常回答だけ返した場合の扱い。
   - v0 は失敗として task event に記録する。
   - 将来、親側が stdout を plan として取り込む fallback を追加できる。

4. `approve` の粒度。
   - v0 は task plan 全体承認。
   - 将来は section 単位承認や option 選択を追加できる。
