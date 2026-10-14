# Nix owns the launch environment and maintenance commands; upstream owns the
# mutable app installations. No network access is needed during activation.
{ config, pkgs, ... }:
let
  # Official installer scripts inspected on 2026-10-14. Changed content must
  # be reviewed and these hashes updated before bootstrap can execute it.
  installerHashes = {
    claude = "3a68d3406cf674e17bed1733a4dcf37805e2e47d87417700007d7e1aa766a944";
    codex = "150e3cf675682efeaac115aa3747add3f27887896d04ce6d0b56478d8b428bf6";
    antigravity = "62966c07365423bd4dc209355060744058fb30d60f5323e2d360e39de64e5042";
  };

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
          echo "Usage: ai-tools-${mode} [all|claude|codex|antigravity]" >&2
          exit 2
        fi
        case "''${1:-all}" in
          all) apps=(claude codex antigravity) ;;
          claude|codex|antigravity) apps=("$1") ;;
          -h|--help)
            echo "Usage: ai-tools-${mode} [all|claude|codex|antigravity]"
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
          case "$app" in
            claude) root="$HOME/.local/share/claude" ;;
            codex) root="''${CODEX_HOME:-$HOME/.codex}/packages/standalone" ;;
            # Installs a single self-updating `agy` binary into ~/.local/bin.
            antigravity)
              launcher="$HOME/.local/bin/agy"
              root="$HOME/.local/bin" ;;
          esac
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
                if ! echo "${installerHashes.claude}  $installer" | sha256sum --check --status; then
                  echo "$app: installer checksum mismatch; review upstream changes before updating the pinned hash." >&2
                  exit 1
                fi
                bash "$installer" || exit 1
              elif [[ "$app" == antigravity ]]; then
                curl -fsSL https://antigravity.google/cli/install.sh -o "$installer" || exit 1
                if ! echo "${installerHashes.antigravity}  $installer" | sha256sum --check --status; then
                  echo "$app: installer checksum mismatch; review upstream changes before updating the pinned hash." >&2
                  exit 1
                fi
                bash "$installer" || exit 1
              else
                curl -fsSL https://chatgpt.com/codex/install.sh -o "$installer" || exit 1
                if ! echo "${installerHashes.codex}  $installer" | sha256sum --check --status; then
                  echo "$app: installer checksum mismatch; review upstream changes before updating the pinned hash." >&2
                  exit 1
                fi
                CODEX_INSTALL_DIR="$HOME/.local/bin" CODEX_NON_INTERACTIVE=1 bash "$installer" || exit 1
              fi
            ); then
              echo "$app: installation failed." >&2
              failed=1
              continue
            fi
          fi

          if [[ "$mode" == update && "$app" == antigravity ]]; then
            echo "$app: self-updates in the background; no manual update step."
          elif [[ "$mode" == update ]]; then
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
