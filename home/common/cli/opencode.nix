{ pkgs, ... }:
{
  home.packages = with pkgs; [
    opencode
  ];

  # OpenCode configuration with Shonan theme
  xdg.configFile."opencode/themes/shonan.json".text = builtins.toJSON {
    "$schema" = "https://opencode.ai/theme.json";
    defs = {
      # Background colors - twilight blue
      base00 = "#1E2433";
      base01 = "#272D3F";
      base02 = "#353D52";
      base03 = "#4A5368";
      # Foreground colors - sky white
      base04 = "#A0B4C8";
      base05 = "#C5D4E8";
      base06 = "#E0EAF5";
      base07 = "#F0F6FC";
      # Accent colors - sunset & sea
      red = "#E8787A";
      orange = "#F0A070";
      yellow = "#E8C87A";
      green = "#8AC4A0";
      cyan = "#70D0E8";
      blue = "#60B8E8";
      purple = "#C090E0";
      pink = "#E890C0";
    };
    theme = {
      primary = {
        dark = "cyan";
        light = "blue";
      };
      secondary = {
        dark = "blue";
        light = "blue";
      };
      accent = {
        dark = "purple";
        light = "purple";
      };
      error = {
        dark = "red";
        light = "red";
      };
      warning = {
        dark = "orange";
        light = "orange";
      };
      success = {
        dark = "green";
        light = "green";
      };
      info = {
        dark = "cyan";
        light = "blue";
      };
      text = {
        dark = "base05";
        light = "base00";
      };
      textMuted = {
        dark = "base03";
        light = "base01";
      };
      background = {
        dark = "base00";
        light = "base07";
      };
      backgroundPanel = {
        dark = "base01";
        light = "base06";
      };
      backgroundElement = {
        dark = "base01";
        light = "base05";
      };
      border = {
        dark = "base02";
        light = "base03";
      };
      borderActive = {
        dark = "base03";
        light = "base02";
      };
      borderSubtle = {
        dark = "base02";
        light = "base03";
      };
      diffAdded = {
        dark = "green";
        light = "green";
      };
      diffRemoved = {
        dark = "red";
        light = "red";
      };
      diffContext = {
        dark = "base03";
        light = "base03";
      };
      diffHunkHeader = {
        dark = "base03";
        light = "base03";
      };
      diffHighlightAdded = {
        dark = "green";
        light = "green";
      };
      diffHighlightRemoved = {
        dark = "red";
        light = "red";
      };
      diffAddedBg = {
        dark = "base01";
        light = "base06";
      };
      diffRemovedBg = {
        dark = "base01";
        light = "base06";
      };
      diffContextBg = {
        dark = "base01";
        light = "base06";
      };
      diffLineNumber = {
        dark = "base02";
        light = "base04";
      };
      diffAddedLineNumberBg = {
        dark = "base01";
        light = "base06";
      };
      diffRemovedLineNumberBg = {
        dark = "base01";
        light = "base06";
      };
      markdownText = {
        dark = "base05";
        light = "base00";
      };
      markdownHeading = {
        dark = "cyan";
        light = "blue";
      };
      markdownLink = {
        dark = "blue";
        light = "blue";
      };
      markdownLinkText = {
        dark = "purple";
        light = "purple";
      };
      markdownCode = {
        dark = "green";
        light = "green";
      };
      markdownBlockQuote = {
        dark = "base03";
        light = "base03";
      };
      markdownEmph = {
        dark = "orange";
        light = "orange";
      };
      markdownStrong = {
        dark = "yellow";
        light = "yellow";
      };
      markdownHorizontalRule = {
        dark = "base03";
        light = "base03";
      };
      markdownListItem = {
        dark = "cyan";
        light = "blue";
      };
      markdownListEnumeration = {
        dark = "purple";
        light = "purple";
      };
      markdownImage = {
        dark = "blue";
        light = "blue";
      };
      markdownImageText = {
        dark = "purple";
        light = "purple";
      };
      markdownCodeBlock = {
        dark = "base05";
        light = "base00";
      };
      syntaxComment = {
        dark = "base03";
        light = "base03";
      };
      syntaxKeyword = {
        dark = "purple";
        light = "purple";
      };
      syntaxFunction = {
        dark = "cyan";
        light = "cyan";
      };
      syntaxVariable = {
        dark = "blue";
        light = "blue";
      };
      syntaxString = {
        dark = "green";
        light = "green";
      };
      syntaxNumber = {
        dark = "orange";
        light = "orange";
      };
      syntaxType = {
        dark = "cyan";
        light = "cyan";
      };
      syntaxOperator = {
        dark = "pink";
        light = "pink";
      };
      syntaxPunctuation = {
        dark = "base05";
        light = "base00";
      };
    };
  };

  # OpenCode config with Shonan theme and auth plugins
  xdg.configFile."opencode/opencode.json".text = builtins.toJSON {
    "$schema" = "https://opencode.ai/config.json";
    theme = "shonan";
    plugin = [
      "opencode-openai-codex-auth@latest"
      "opencode-gemini-auth@latest"
    ];
  };
}
