{
  config,
  pkgs,
  lib,
  ...
}:
let
  inherit (pkgs.stdenv) isDarwin;
in
{
  # Claude Code 本体は公式 curl インストーラで管理する。
  # - インストール先: ~/.local/bin/claude (→ ~/.local/share/claude/versions/<ver>)
  # - 更新: Claude Code 自身が自動更新するので activation では再実行しない
  # - Nix / brew / npm では入れない (公式バイナリを単一の真実とする)
  home.packages =
    with pkgs;
    lib.optionals (!isDarwin) [
      chromium # For Playwright MCP (Linux only)
    ];

  home.activation.installClaudeCode = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    if [ ! -x "$HOME/.local/bin/claude" ]; then
      echo "Installing Claude Code via official installer..."
      # install.sh re-invokes curl/wget from PATH, which is sanitized during
      # home-manager activation — make curl available to the piped script.
      PATH="${pkgs.curl}/bin:$PATH" ${pkgs.curl}/bin/curl -fsSL https://claude.ai/install.sh | PATH="${pkgs.curl}/bin:$PATH" bash
    fi
  '';

  # /skill-sync — スキル正本ライブラリ (claude/skills/) の同期スキルだけは
  # Nix で ~/.claude/skills に symlink する(「~/.claude は管理しない」方針の例外)。
  # out-of-store symlink なので rebuild なしで編集が反映される。
  # 他のスキルの配布は `dev skill push` が行う (claude/skills/skills.toml 参照)。
  home.file.".claude/skills/skill-sync".source =
    config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/claude/skills/skill-sync";

  # /fleet-review — agy/codex/opencode の3系統を並列実行して統合レビューする
  # グローバルスキル。skill-sync 同様、全マシンに配るので home-manager で
  # out-of-store symlink する（project 配布ではないため skills.toml は projects=[]）。
  home.file.".claude/skills/fleet-review".source =
    config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/claude/skills/fleet-review";

  # fleet-review が使う reviewer サブエージェント定義。ファイル単位で symlink し、
  # マシン固有のローカル agent を ~/.claude/agents に共存できる余地を残す。
  home.file.".claude/agents/agy-reviewer.md".source =
    config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/claude/agents/agy-reviewer.md";
  home.file.".claude/agents/codex-reviewer.md".source =
    config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/claude/agents/codex-reviewer.md";
  home.file.".claude/agents/opencode-reviewer.md".source =
    config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/claude/agents/opencode-reviewer.md";

  # Claude Code settings.json — Nix管理しない
  # Claude Codeがpermissions・プラグイン設定を頻繁に書き換えるため、各マシンで独立管理。
  # settings.local.json と合わせて手動で管理する。

  # MCP servers: Nix管理しない
  # claude mcp add で ~/.claude.json に追加して各マシンで独立管理する。
}
