{
  pkgs,
  lib,
  ...
}:
let
  inherit (pkgs.stdenv) isDarwin;
in
{
  # Claude Code本体:
  # - macOS: brew install claude-code
  # - Linux: npm install -g @anthropic-ai/claude-code (activation で自動インストール)
  home.packages =
    with pkgs;
    lib.optionals (!isDarwin) [
      chromium # For Playwright MCP (Linux only)
    ];

  # Linux環境では npm で Claude Code をインストール・更新
  # Nix store は read-only なので、npm prefix をユーザーディレクトリに設定
  home.activation.installClaudeCode = lib.mkIf (!isDarwin) (
    lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      export PATH="${pkgs.nodejs}/bin:$PATH"
      export NPM_CONFIG_PREFIX="$HOME/.npm-global"
      mkdir -p "$HOME/.npm-global"
      if ! "$HOME/.npm-global/bin/claude" --version &>/dev/null 2>&1; then
        echo "Installing Claude Code via npm..."
        ${pkgs.nodejs}/bin/npm install -g @anthropic-ai/claude-code
      fi
    ''
  );

  # Claude Code settings.json — Nix管理しない
  # Claude Codeがpermissions・プラグイン設定を頻繁に書き換えるため、各マシンで独立管理。
  # settings.local.json と合わせて手動で管理する。

  # MCP servers: Nix管理しない
  # claude mcp add で ~/.claude.json に追加して各マシンで独立管理する。
}
