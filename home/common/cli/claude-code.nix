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
  # Claude Code 本体は ai-tools-bootstrap で公式インストーラから導入する。
  # - インストール先: ~/.local/bin/claude (→ ~/.local/share/claude/versions/<ver>)
  # - 更新: 自己更新、または ai-tools-update claude。activation では取得しない
  # - Nix / brew / npm では入れない (公式バイナリを単一の真実とする)
  home.packages =
    with pkgs;
    lib.optionals (!isDarwin) [
      chromium # For Playwright MCP (Linux only)
    ];

  # Remove only the statusline installed by the retired dev integration.
  home.activation.removeDevStatusline = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    settings="$HOME/.claude/settings.json"
    if [[ -f "$settings" ]] && ${pkgs.jq}/bin/jq -e '
      .statusLine.type == "command" and
      ((.statusLine.command // "") | test("^/nix/store/[^/]+-dev-statusline/bin/dev-statusline$"))
    ' "$settings" >/dev/null; then
      if [[ -z "''${DRY_RUN_CMD:-}" ]]; then
        tmp=$(${pkgs.coreutils}/bin/mktemp "$settings.XXXXXX")
        if ${pkgs.jq}/bin/jq 'del(.statusLine)' "$settings" > "$tmp"; then
          ${pkgs.coreutils}/bin/mv "$tmp" "$settings"
        else
          ${pkgs.coreutils}/bin/rm -f "$tmp"
          exit 1
        fi
      fi
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
