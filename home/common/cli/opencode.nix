{
  pkgs,
  lib,
  config,
  ...
}:
let
  inherit (pkgs.stdenv) isDarwin;
  localConfig = "${config.xdg.configHome}/opencode/local.json";
in
{
  # macOS では Homebrew、Linux では Nix pkgs で管理
  home.packages =
    with pkgs;
    lib.optionals (!isDarwin) [
      opencode
    ];

  # Secrets we don't want in the public repo (vLLM baseURL + apiKey, …) live in
  # an untracked, hand-written local.json that opencode deep-merges on top of the
  # global opencode.json below (via OPENCODE_CONFIG). This file is intentionally
  # NOT managed by home-manager — that would push its contents into the public
  # repo and the world-readable /nix/store. Only the pointer below is in Nix.
  # Create/edit it by hand; it lives in ~/.config, outside ~/dotfiles:
  #   ~/.config/opencode/local.json   (chmod 600)
  #   { "provider": { "vllm": { "npm": "@ai-sdk/openai-compatible",
  #       "options": { "baseURL": "...", "apiKey": "..." },
  #       "models": { "<model-id>": {} } } } }
  home.sessionVariables.OPENCODE_CONFIG = localConfig;
  programs.zsh.initContent = lib.mkBefore ''
    export OPENCODE_CONFIG="${localConfig}"
  '';

  # OpenCode configuration with Shonan theme
  xdg.configFile."opencode/themes/shonan.json".text = builtins.toJSON (
    import ./opencode/shonan-theme.nix
  );

  # OpenCode config with Shonan theme and auth plugins
  xdg.configFile."opencode/opencode.json".text = builtins.toJSON {
    "$schema" = "https://opencode.ai/config.json";
    theme = "shonan";
    plugin = [
      "opencode-openai-codex-auth@latest"
      "opencode-gemini-auth@latest"
    ];
    # anthropic/google accessed via opencode-openai-codex-auth/opencode-gemini-auth plugins above
    disabled_providers = [
      "anthropic"
      "google"
    ];
  };
}
