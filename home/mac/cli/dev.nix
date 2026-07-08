{
  inputs,
  pkgs,
  lib,
  config,
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

  # Mutable, out-of-store copy of the zellij board wasm so `dev board` can be
  # overridden without a switch (seeded by activation.devZellijWasm below).
  wasmTarget = "${config.home.homeDirectory}/.local/state/dev/dev.wasm";

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
  # are built by the standalone dev flake and consumed here as packages — but
  # only as the *fallback* build. The wrappers below prefer a locally built
  # artifact so active iteration on ~/git/dev needs no `nh switch`.
  devPkgs = inputs.dev.packages.${pkgs.stdenv.hostPlatform.system};
  inherit (devPkgs) dev-tui dev-zellij;

  # Active-development override. While iterating on ~/git/dev, a locally
  # cargo-built binary shadows the flake-pinned one so a plain `cargo build`
  # (or `just build`) is picked up immediately — no commit, no `nh switch`.
  # This snippet sets $bin to: $<envVar> (explicit) → target/release/<name> →
  # target/debug/<name> → the nix fallback. Delete the target dir (or unset the
  # env var) to fall back to the pinned build.
  resolveLocal =
    {
      name,
      envVar,
      fallback,
    }:
    ''
      bin="''${${envVar}:-}"
      if [[ -z "$bin" ]]; then
        for candidate in "$HOME/git/dev/target/release/${name}" "$HOME/git/dev/target/debug/${name}"; do
          [[ -x "$candidate" ]] && { bin="$candidate"; break; }
        done
      fi
      [[ -n "$bin" && -x "$bin" ]] || bin="${fallback}"
    '';

  # `dev` is the Rust binary from the standalone dev flake. This thin wrapper
  # only sets up the runtime env the binary expects — the SSH agent socket and
  # secrets sourced from local.zsh (Telegram token) — then execs the resolved
  # binary. All fleet logic (target resolution, SSH, git, agent lifecycle,
  # TUI/board data) lives in Rust, in the dev repo.
  devCmd = pkgs.writeShellScriptBin "dev" ''
    ${loadConfig}
    ${resolveLocal {
      name = "dev";
      envVar = "DEV_LOCAL_BIN";
      fallback = "${devPkgs.dev-cli}/bin/dev";
    }}
    exec "$bin" "$@"
  '';

  # `dev tui` execs `dev-tui` off PATH (dev-cli/src/cmd/launch.rs); this wrapper
  # is that PATH entry, so the TUI also rides the local-build override. It needs
  # the same runtime env as `dev` (SSH agent + secrets to launch sessions).
  devTuiCmd = pkgs.writeShellScriptBin "dev-tui" ''
    ${loadConfig}
    ${resolveLocal {
      name = "dev-tui";
      envVar = "DEV_LOCAL_TUI_BIN";
      fallback = "${dev-tui}/bin/dev-tui";
    }}
    exec "$bin" "$@"
  '';

  # Claude Code's statusLine hook (see below) points here instead of at the raw
  # dev-cli so statusline code changes are switchless too. No loadConfig — the
  # per-response hook stays a single fast exec (statusline needs no secrets/SSH).
  devStatuslineCmd = pkgs.writeShellScriptBin "dev-statusline" ''
    ${resolveLocal {
      name = "dev";
      envVar = "DEV_LOCAL_BIN";
      fallback = "${devPkgs.dev-cli}/bin/dev";
    }}
    exec "$bin" statusline "$@"
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
      devTuiCmd
      opencodeWrapper
    ];

    file = {
      # Zellij task-board plugin. `dev board` loads this fixed path, so to keep
      # the board switchless it's an out-of-store symlink to a mutable location
      # (~/.local/state/dev/dev.wasm) that the activation below seeds from the
      # nix build. Drop a locally built wasm there to override with no switch:
      #   nix build ~/git/dev#dev-zellij -o /tmp/w \
      #     && install -m444 /tmp/w ~/.local/state/dev/dev.wasm
      # (cargo can't build it — it's a wasm32-wasi cross build; see dev/default.nix.)
      ".config/zellij/plugins/dev.wasm".source = config.lib.file.mkOutOfStoreSymlink wasmTarget;
    };

    # Seed the mutable out-of-store zellij wasm (see home.file above) from the
    # nix build only when absent, so a locally built wasm dropped there survives
    # switches. `rm` it to re-seed with the pinned build on the next switch.
    activation.devZellijWasm = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      if [[ ! -e "${wasmTarget}" ]]; then
        run mkdir -p "$(dirname "${wasmTarget}")"
        run install -m444 ${dev-zellij} "${wasmTarget}"
      fi
    '';

    # Inject statusLine into ~/.claude/settings.json (merged, not replaced), pointing
    # at the dev-statusline wrapper — which resolves the local build (switchless)
    # yet stays a single fast exec (no loadConfig). It reads Claude Code's JSON,
    # refreshes the ~/.cache/claude/usage.json cache (for `dev usage` / the TUI),
    # and prints the 5h/7d bar. Replaces the old bash+jq ~/.claude/statusline.sh;
    # normalization lives in dev-core::statusline. Claude Code writes settings.json
    # itself, so we merge at switch time (idempotent, survives its own writes)
    # rather than managing it as a symlink.
    activation.claudeStatusline = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      settings="$HOME/.claude/settings.json"
      cmd="${devStatuslineCmd}/bin/dev-statusline"
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
