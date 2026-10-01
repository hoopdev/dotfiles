{
  pkgs,
  lib,
  ...
}:
let
  inherit (pkgs.stdenv.hostPlatform) isDarwin;
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
  };

  # Claude Code settings.json — Nix管理しない
  # Claude Codeがpermissions・プラグイン設定を頻繁に書き換えるため、各マシンで独立管理。
  # settings.local.json と合わせて手動で管理する。

  # MCP servers and agent hooks: Orca owns their integration where supported.
  # Keep credentials and mutable agent settings outside Nix. Migrate existing
  # registrations through Orca Settings, verify on the execution host, then
  # remove duplicates. See docs/orca.md; do not overwrite ~/.claude.json here.
}
