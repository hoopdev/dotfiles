{ pkgs, config, ... }:
{
  home.packages = with pkgs; [
    claude-code
    chromium # For Playwright MCP
  ];

  # Claude Code settings.json (user-level settings)
  home.file.".claude/settings.json".text = builtins.toJSON {
    # Permission rules for frequently used tools
    permissions = {
      allow = [
        "Bash(git:*)"
        "Bash(nix:*)"
        "Bash(nixos-rebuild:*)"
        "Bash(darwin-rebuild:*)"
        "Bash(home-manager:*)"
        "Bash(npm:*)"
        "Bash(pnpm:*)"
        "Bash(cargo:*)"
        "Bash(python:*)"
        "Bash(uv:*)"
        "Bash(ls:*)"
        "Bash(readlink:*)"
        "Bash(wezterm:*)"
        "Bash(grep:*)"
        "Bash(cat:*)"
        "Read"
        "Write"
        "Edit"
        "Glob"
        "Grep"
        "WebSearch"
      ];
      deny = [ ];
    };

    # Preferred settings
    theme = "dark-daltonized";
    autoUpdates = false; # Managed by Nix

    # MCP server enable/disable settings
    # enableAllProjectMcpServers = true;
    # enabledMcpjsonServers = [ "server-name" ];
  };

  # Global MCP servers configuration
  # Note: claude mcp add で追加したサーバーは ~/.claude.json に保存される
  # この設定ファイルを使う場合は claude --mcp-config ~/.claude/mcp.json で起動
  home.file.".claude/mcp.json".text = builtins.toJSON {
    mcpServers = {
      filesystem = {
        type = "stdio";
        command = "npx";
        args = [ "-y" "@modelcontextprotocol/server-filesystem" config.home.homeDirectory ];
      };

      playwright = {
        type = "stdio";
        command = "npx";
        args = [
          "-y"
          "@playwright/mcp@latest"
          "--executable-path"
          "${pkgs.chromium}/bin/chromium"
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
        args = [ "-y" "@upstash/context7-mcp" ];
      };
    };
  };
}
