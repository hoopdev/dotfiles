# Shonan theme for opencode, built from the Stylix base16 palette.
# Consumed as: import ./opencode/shonan-theme.nix { inherit lib; inherit (config.lib.stylix) colors; }
# `colors.baseXX` are bare lowercase hex strings (no "#"), as in
# shell/starship.nix; upper-cased here to match the previous hand-written file.
{ colors, lib }:
let
  hex = name: "#${lib.toUpper colors.${name}}";
in
{
  "$schema" = "https://opencode.ai/theme.json";
  defs = {
    # Background colors - twilight blue
    base00 = hex "base00";
    base01 = hex "base01";
    base02 = hex "base02";
    base03 = hex "base03";
    # Foreground colors - sky white
    base04 = hex "base04";
    base05 = hex "base05";
    base06 = hex "base06";
    base07 = hex "base07";
    # Accent colors - sunset & sea
    red = hex "base08";
    orange = hex "base09";
    yellow = hex "base0A";
    green = hex "base0B";
    cyan = hex "base0C";
    blue = hex "base0D";
    purple = hex "base0E";
    pink = hex "base0F";
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
}
