{
  inputs,
  pkgs,
  lib,
  ...
}:
# Everything the `dev` fleet tool needs, kept out of coder.nix (which is coder
# connection only): the `dev` CLI + TUI + Zellij board, the opencode wrapper
# that `dev` agents launch, and the Claude Code statusline that feeds
# `dev usage` / the TUI.
let
  # local.zsh (untracked) holds only the secrets these wrappers source:
  # TELEGRAM_* for `dev notify`, LLM_CF_TOKEN for opencode. Fleet topology
  # (envs / local / remote projects) lives in ~/.config/dev/config.toml, managed
  # by `dev config`; migrate old DEV_* arrays with
  # `dev config import`.
  localZsh = "$HOME/.config/zsh/local.zsh";

  # jq only injects statusLine into Claude Code's settings.json at switch time
  # (a one-shot merge; the per-response cache is now written by `dev statusline`).
  jq = "${pkgs.jq}/bin/jq";

  loadConfig = ''
    export PATH="$HOME/.nix-profile/bin:$PATH"
    [[ -f "${localZsh}" ]] && source "${localZsh}"
    # SSH agent: $SSH_AUTH_SOCK is the single source of truth (see
    # home/common/cli/ssh.nix). Non-interactive `dev` invocations (subagents)
    # may not have run the macOS login hook, so point it at the 1Password agent
    # when unset for local invocations. SSH sessions should use only a
    # client-forwarded agent so the origin machine controls key approval.
    if [[ -z "''${SSH_AUTH_SOCK:-}" && -z "''${SSH_CONNECTION:-}" ]]; then
      export SSH_AUTH_SOCK="''${DEV_SSH_AGENT_SOCK:-$HOME/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock}"
    fi
  '';

  # dev-cli (`dev`), dev-tui (`dev tui`) and dev-zellij (`dev board`, Wasm)
  # are built by the standalone dev flake and consumed here as packages.
  devPkgs = inputs.dev.packages.${pkgs.stdenv.hostPlatform.system};
  inherit (devPkgs) dev-tui dev-zellij;

  # `dev` is the Rust binary from the standalone dev flake. This thin wrapper
  # only sets up the runtime env the binary expects — the SSH agent socket and
  # secrets sourced from local.zsh (Telegram token) — then execs it. All fleet
  # logic (target resolution, SSH, git, agent lifecycle, TUI/board data) lives
  # in Rust, in the dev repo.
  devCmd = pkgs.writeShellScriptBin "dev" ''
    ${loadConfig}
    exec ${devPkgs.dev-cli}/bin/dev "$@"
  '';

  # opencode is launched by `dev` agents; this wrapper sources local.zsh so
  # LLM_CF_TOKEN (and other secrets) are present even when a subagent invokes it
  # without an interactive-shell env.
  opencodeWrapper = pkgs.writeShellScriptBin "opencode" ''
    export PATH="$HOME/.nix-profile/bin:/opt/homebrew/bin:$PATH"
    [[ -f "${localZsh}" ]] && source "${localZsh}"
    exec /opt/homebrew/bin/opencode "$@"
  '';
in
{
  home = {
    packages = [
      devCmd
      dev-tui
      opencodeWrapper
    ];

    file = {
      # Zellij task-board plugin — installed to the standard plugin search path.
      # Load with: dev board  (or: zellij plugin -- file:~/.config/zellij/plugins/dev.wasm)
      ".config/zellij/plugins/dev.wasm".source = dev-zellij;
    };

    # Inject statusLine into ~/.claude/settings.json (merged, not replaced), pointing
    # at the raw `dev statusline` binary (dev-cli) — no wrapper, so the per-response
    # hook is a single fast exec that reads Claude Code's JSON, refreshes the
    # ~/.cache/claude/usage.json cache (for `dev usage` / the TUI), and prints the
    # 5h/7d bar. Replaces the old bash+jq ~/.claude/statusline.sh; normalization now
    # lives in dev-core::statusline. Claude Code writes settings.json itself, so we
    # merge at switch time (idempotent, survives Claude Code's own writes) rather
    # than managing it as a symlink.
    activation.claudeStatusline = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      settings="$HOME/.claude/settings.json"
      cmd="${devPkgs.dev-cli}/bin/dev statusline"
      if [[ -f "$settings" ]]; then
        run ${jq} --arg cmd "$cmd" \
          '. + {statusLine: {type: "command", command: $cmd}}' \
          "$settings" > "$settings.tmp" && mv "$settings.tmp" "$settings"
      else
        mkdir -p "$(dirname "$settings")"
        run ${jq} -n --arg cmd "$cmd" \
          '{statusLine: {type: "command", command: $cmd}}' \
          > "$settings"
      fi
    '';
  };
}
