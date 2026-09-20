# Nix owns the launch environment and maintenance commands; upstream owns the
# mutable app installations. No network access is needed during activation.
{ config, pkgs, ... }:
let
  mkCommand =
    mode:
    pkgs.writeShellApplication {
      name = "ai-tools-${mode}";
      runtimeInputs = with pkgs; [
        bash
        curl
        coreutils
        gnugrep
        gnused
        gawk
        gnutar
        gzip
        jq
      ];
      text = ''
        mode=${mode}
        if [[ $# -gt 1 ]]; then
          echo "Usage: ai-tools-${mode} [all|claude|codex]" >&2
          exit 2
        fi
        case "''${1:-all}" in
          all) apps=(claude codex) ;;
          claude|codex) apps=("$1") ;;
          -h|--help)
            echo "Usage: ai-tools-${mode} [all|claude|codex]"
            echo "Manage official user-local installations; running services are not restarted."
            exit 0 ;;
          *) echo "Unknown app: $1" >&2; exit 2 ;;
        esac

        # The official installers use this directory. Also prevents them from
        # needing to edit shell startup files to configure PATH.
        export PATH="$HOME/.local/bin:$PATH"
        failed=0
        for app in "''${apps[@]}"; do
          launcher="$HOME/.local/bin/$app"
          if [[ "$app" == claude ]]; then
            root="$HOME/.local/share/claude"
          else
            root="''${CODEX_HOME:-$HOME/.codex}/packages/standalone"
          fi
          if [[ -e "$launcher" || -L "$launcher" ]]; then
            resolved=$(realpath -m "$launcher")
            case "$resolved" in
              "$(realpath -m "$root")"/*) ;;
              *)
                echo "$app: $launcher is not an official native installation ($resolved)." >&2
                echo "Use its owning installer, or migrate it before using this command." >&2
                failed=1
                continue ;;
            esac
            if [[ ! -x "$launcher" ]]; then
              echo "$app: broken launcher at $launcher; repair the native installation first." >&2
              failed=1
              continue
            fi
          else
            if existing=$(command -v "$app"); then
              echo "$app: already installed at $existing; use its owning installer or migrate first." >&2
              failed=1
              continue
            fi
            if [[ "$mode" == update ]]; then
              echo "$app: not installed. Run ai-tools-bootstrap $app first." >&2
              failed=1
              continue
            fi

            # Download completely before executing; a failed download must never
            # execute a partial installer. Each subshell cleans up its own file.
            if ! (
              installer=$(mktemp)
              trap 'rm -f "$installer"' EXIT
              if [[ "$app" == claude ]]; then
                curl -fsSL https://claude.ai/install.sh -o "$installer" || exit 1
                bash "$installer" || exit 1
              else
                curl -fsSL https://chatgpt.com/codex/install.sh -o "$installer" || exit 1
                CODEX_INSTALL_DIR="$HOME/.local/bin" CODEX_NON_INTERACTIVE=1 bash "$installer" || exit 1
              fi
            ); then
              echo "$app: installation failed." >&2
              failed=1
              continue
            fi
          fi

          if [[ "$mode" == update ]]; then
            if ! "$launcher" update; then
              echo "$app: update failed." >&2
              failed=1
              continue
            fi
          fi
          if ! "$launcher" --version; then
            echo "$app: cannot run; on NixOS check nix-ld and its libraries." >&2
            failed=1
            continue
          fi

          if [[ "$app" == codex ]]; then
            if status=$("$launcher" app-server daemon version 2>/dev/null); then
              if jq -e '.appServerVersion != null and .cliVersion != null and .appServerVersion != .cliVersion' \
                <<< "$status" >/dev/null 2>&1; then
                echo "Codex service version differs from the CLI. Restart it through its owner after active work finishes."
                echo "For a CLI-managed daemon: codex app-server daemon restart"
              fi
            else
              echo "Codex service status unavailable; no service was restarted." >&2
            fi
          fi
        done
        exit "$failed"
      '';
    };
in
{
  home.sessionPath = [ "${config.home.homeDirectory}/.local/bin" ];
  home.packages = map mkCommand [
    "bootstrap"
    "update"
  ];
}
