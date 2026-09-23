{
  config,
  pkgs,
  lib,
  ...
}:
let
  inherit (pkgs.stdenv.hostPlatform) isDarwin;
  repo = config.dotfiles.paths.repo;
  # out-of-store symlink into the dotfiles checkout; rebuild-free edits.
  fromRepo = path: config.lib.file.mkOutOfStoreSymlink "${repo}/${path}";
in
{
  # Claude Code 本体は ai-tools-bootstrap で公式インストーラから導入する。
  # - インストール先: ~/.local/bin/claude (→ ~/.local/share/claude/versions/<ver>)
  # - 更新: 自己更新、または ai-tools-update claude。activation では取得しない
  # - Nix / brew / npm では入れない (公式バイナリを単一の真実とする)

  home = {
    packages =
      with pkgs;
      lib.optionals (!isDarwin) [
        chromium # For Playwright MCP (Linux only)
      ];

    # Remove only the statusline installed by the retired dev integration.
    activation.removeDevStatusline = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
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

    # The symlinks below point into the checkout named by dotfiles.paths.repo
    # (host metadata `paths.repo`; default ~/dotfiles). Skipped when the host
    # declares no checkout, exactly like the init.lua sync in neovim.nix.
    file = lib.mkIf (repo != null) {
      # /skill-sync — スキル正本ライブラリ (claude/skills/) の同期スキルだけは
      # Nix で ~/.claude/skills に symlink する(「~/.claude は管理しない」方針の例外)。
      # out-of-store symlink なので rebuild なしで編集が反映される。
      # 他のスキルの配布は `dev skill push` が行う (claude/skills/skills.toml 参照)。
      ".claude/skills/skill-sync".source = fromRepo "claude/skills/skill-sync";

      # /fleet-review — agy/codex/opencode の3系統を並列実行して統合レビューする
      # グローバルスキル。skill-sync 同様、全マシンに配るので home-manager で
      # out-of-store symlink する(project 配布ではないため skills.toml は projects=[])。
      ".claude/skills/fleet-review".source = fromRepo "claude/skills/fleet-review";

      # fleet-review が使う reviewer サブエージェント定義。ファイル単位で symlink し、
      # マシン固有のローカル agent を ~/.claude/agents に共存できる余地を残す。
      ".claude/agents/agy-reviewer.md".source = fromRepo "claude/agents/agy-reviewer.md";
      ".claude/agents/codex-reviewer.md".source = fromRepo "claude/agents/codex-reviewer.md";
      ".claude/agents/opencode-reviewer.md".source = fromRepo "claude/agents/opencode-reviewer.md";
    };
  };

  # Claude Code settings.json — Nix管理しない
  # Claude Codeがpermissions・プラグイン設定を頻繁に書き換えるため、各マシンで独立管理。
  # settings.local.json と合わせて手動で管理する。

  # MCP servers: Nix管理しない
  # claude mcp add で ~/.claude.json に追加して各マシンで独立管理する。
}
