{
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

  # Claude Code settings.json — Nix管理しない
  # Claude Codeがpermissions・プラグイン設定を頻繁に書き換えるため、各マシンで独立管理。
  # settings.local.json と合わせて手動で管理する。

  # MCP servers: Nix管理しない
  # claude mcp add で ~/.claude.json に追加して各マシンで独立管理する。
}
