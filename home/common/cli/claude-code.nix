{
  pkgs,
  config,
  lib,
  ...
}:
let
  isDarwin = pkgs.stdenv.isDarwin;
  # macOS uses Google Chrome from Applications, Linux uses Chromium from nixpkgs
  chromePath =
    if isDarwin then
      "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
    else
      "${pkgs.chromium}/bin/chromium";
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

  # Global MCP servers configuration
  # Note: claude mcp add で追加したサーバーは ~/.claude.json に保存される
  # この設定ファイルを使う場合は claude --mcp-config ~/.claude/mcp.json で起動
  home.file.".claude/mcp.json".text = builtins.toJSON {
    mcpServers = {
      filesystem = {
        type = "stdio";
        command = "npx";
        args = [
          "-y"
          "@modelcontextprotocol/server-filesystem"
          config.home.homeDirectory
        ];
      };

      playwright = {
        type = "stdio";
        command = "npx";
        args = [
          "-y"
          "@playwright/mcp@latest"
          "--executable-path"
          chromePath
        ];
        env = {
          PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = "1";
          PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
        };
      };

      github = {
        type = "stdio";
        command = "bash";
        args = [
          "-c"
          "GITHUB_PERSONAL_ACCESS_TOKEN=$(gh auth token) npx -y @modelcontextprotocol/server-github"
        ];
      };

      context7 = {
        type = "stdio";
        command = "npx";
        args = [
          "-y"
          "@upstash/context7-mcp"
        ];
      };
    };
  };
}
