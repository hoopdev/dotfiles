{ pkgs, config, lib, ... }:
let
  coderSessionPath = "${config.home.homeDirectory}/Library/Application Support/coderv2/session";
  # Runtime config lives in ~/.config/zsh/local.zsh (not tracked in git).
  #
  # DEV_ENVS   — SSH connection targets: "name|user@host|proxy_cmd|shell|os"
  #              proxy_cmd: empty = direct SSH; %h = hostname placeholder.
  #              shell: bash (default), zsh, pwsh, nu
  #              os: optional display label (linux, macos, windows, ...)
  # DEV_LOCAL  — Local projects:  "name|path"
  # DEV_REMOTE — Remote projects: "name|env_name|remote_path"
  #              env_name must be a name in DEV_ENVS.
  # DEV_SSH_AGENT — env names whose SSH forwards the local agent onward (so `ssh`
  #              from the remote reuses local keys). The agent itself is whatever
  #              $SSH_AUTH_SOCK points at (1Password); DEV_SSH_AGENT_SOCK overrides
  #              the socket loadConfig falls back to when SSH_AUTH_SOCK is unset.
  #
  # Example:
  #   DEV_ENVS=(
  #     "myenv|user@myenv.example.com|coder-proxy %h|bash|linux"
  #     "win-machine|user@win.ts.net||nu|windows"
  #   )
  #   DEV_SSH_AGENT=( myenv )
  #   DEV_LOCAL=(
  #     "myproject-local|$HOME/git/myproject"
  #   )
  #   DEV_REMOTE=(
  #     "myproject-server|myenv|/home/user/myproject"
  #     "win-proj|win-machine|C:/Users/user/project"
  #   )
  localZsh = "$HOME/.config/zsh/local.zsh";

  # Hermetic deps (no reliance on the user's profile/PATH): jq parses
  # `claude agents --json` and builds all `--json` output; curl posts Telegram.
  jq = "${pkgs.jq}/bin/jq";
  curl = "${pkgs.curl}/bin/curl";


  loadEnv = ''
    if [[ -z "$CODER_URL" ]]; then
      [[ -f "${localZsh}" ]] && source "${localZsh}"
    fi
    if [[ -z "$CODER_URL" ]]; then
      echo "coder-*: CODER_URL not set — add Coder vars to ${localZsh}" >&2
      exit 1
    fi
  '';

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

  devProjectFns = ''
    # --- DEV_LOCAL helpers ---
    _local_get_path() {
      local target="$1" entry n path
      for entry in "''${DEV_LOCAL[@]:-}"; do
        IFS='|' read -r n path <<< "$entry"
        [[ "$n" == "$target" ]] && { echo "$path"; return 0; }
      done; return 1
    }

    # --- DEV_REMOTE helpers ---
    _remote_get_env() {
      local target="$1" entry n env rp
      for entry in "''${DEV_REMOTE[@]:-}"; do
        IFS='|' read -r n env rp <<< "$entry"
        [[ "$n" == "$target" ]] && { echo "$env"; return 0; }
      done; return 1
    }
    _remote_get_path() {
      local target="$1" entry n env rp
      for entry in "''${DEV_REMOTE[@]:-}"; do
        IFS='|' read -r n env rp <<< "$entry"
        [[ "$n" == "$target" ]] && { echo "$rp"; return 0; }
      done; return 1
    }

    # --- DEV_ENVS helpers ---
    _env_get_host() {
      local target="$1" entry n host proxy shell os
      for entry in "''${DEV_ENVS[@]:-}"; do
        IFS='|' read -r n host proxy shell os <<< "$entry"
        [[ "$n" == "$target" ]] && { echo "$host"; return 0; }
      done; return 1
    }
    _env_get_proxy() {
      local target="$1" entry n host proxy shell os
      for entry in "''${DEV_ENVS[@]:-}"; do
        IFS='|' read -r n host proxy shell os <<< "$entry"
        [[ "$n" == "$target" ]] && { echo "$proxy"; return 0; }
      done; return 0
    }
    _env_get_shell() {
      local target="$1" entry n host proxy shell os
      for entry in "''${DEV_ENVS[@]:-}"; do
        IFS='|' read -r n host proxy shell os <<< "$entry"
        [[ "$n" == "$target" ]] && { echo "''${shell:-bash}"; return 0; }
      done; echo "bash"
    }
    _env_get_os() {
      local target="$1" path="''${2:-}" entry n host proxy shell os rn renv rp
      for entry in "''${DEV_ENVS[@]:-}"; do
        IFS='|' read -r n host proxy shell os <<< "$entry"
        [[ "$n" != "$target" ]] && continue
        if [[ -n "$os" ]]; then echo "$os"; return 0; fi
        case "''${shell:-bash}" in
          pwsh) echo windows; return 0 ;;
        esac
        if [[ "$path" =~ ^[A-Za-z]:[/\\] ]]; then echo windows; return 0; fi
        for entry in "''${DEV_REMOTE[@]:-}"; do
          IFS='|' read -r rn renv rp <<< "$entry"
          if [[ "$renv" == "$target" && "$rp" =~ ^[A-Za-z]:[/\\] ]]; then
            echo windows; return 0
          fi
        done
        echo unknown; return 0
      done
      echo unknown
    }
    _env_exists() {
      local target="$1" entry n host proxy shell os
      for entry in "''${DEV_ENVS[@]:-}"; do
        IFS='|' read -r n host proxy shell os <<< "$entry"
        [[ "$n" == "$target" ]] && return 0
      done; return 1
    }

    _dev_list_projects() {
      local entry n x
      for entry in "''${DEV_LOCAL[@]:-}";  do IFS='|' read -r n x     <<< "$entry"; echo "$n"; done
      for entry in "''${DEV_REMOTE[@]:-}"; do IFS='|' read -r n x x   <<< "$entry"; echo "$n"; done
    }
    _dev_list_envs() {
      local entry n x
      for entry in "''${DEV_ENVS[@]:-}"; do IFS='|' read -r n x x x x <<< "$entry"; echo "$n"; done
    }

    # Shared fzf flags. The preview calls `dev _preview {}` to show the resolved
    # target live as you move the cursor (static info only — no remote git/SSH —
    # so it stays snappy on every keystroke).
    _dev_fzf=(--height=60% --reverse --preview 'dev _preview {}' --preview-window=right,52%,wrap)

    _dev_select_project() {
      if ! command -v fzf >/dev/null 2>&1; then
        echo "dev: fzf is required when project name is omitted" >&2
        return 1
      fi
      _dev_list_projects | fzf "''${_dev_fzf[@]}" --prompt='dev project> '
    }
    _dev_select_any() {
      if ! command -v fzf >/dev/null 2>&1; then
        echo "dev: fzf is required when name is omitted" >&2
        return 1
      fi
      {
        for n in $(_dev_list_projects); do echo "project $n"; done
        for n in $(_dev_list_envs); do echo "env     $n"; done
      } | fzf "''${_dev_fzf[@]}" --prompt='dev> ' | awk '{print $2}'
    }
    _dev_select_many_projects() {
      if ! command -v fzf >/dev/null 2>&1; then
        echo "dev: fzf is required for '-' selection" >&2
        return 1
      fi
      _dev_list_projects | fzf --multi "''${_dev_fzf[@]}" --prompt='dev projects> '
    }

    _dev_require_name() {
      local kind="$1" name="''${2:-}"
      if [[ -n "$name" && "$name" != "-" ]]; then
        echo "$name"
        return 0
      fi
      if [[ "$kind" == "project" ]]; then
        _dev_select_project
      else
        _dev_select_any
      fi
    }

    # Low-level SSH runner.
    # Args: env_name  rp (may be empty)  cmd (may be empty)  interactive (any non-empty = yes)
    # - rp non-empty → cd/Set-Location before running cmd
    # - cmd empty + interactive → opens a login shell
    _dev_exec_on_env() {
      local env_name="$1" rp="''${2:-}" cmd="''${3:-}" interactive="''${4:-}"
      local ssh_host proxy shell flag
      ssh_host=$(_env_get_host  "$env_name") || { echo "dev: unknown env '$env_name'" >&2; return 1; }
      proxy=$(_env_get_proxy    "$env_name")
      shell=$(_env_get_shell    "$env_name")
      flag="-T"; [[ -n "$interactive" ]] && flag="-t"
      # ControlMaster pools dev-initiated SSH: the first connection runs the
      # (expensive) coder-proxy/cloudflared handshake, later `dev` calls
      # (info/status/ps fan-out, shells) reuse it — also collapsing repeated
      # 1Password prompts to one. Scoped here so it doesn't affect other ssh use.
      local -a ssh_opts=(
        -o StrictHostKeyChecking=accept-new
        -o "UserKnownHostsFile=~/.ssh/known_hosts.coder"
        -o ControlMaster=auto
        -o "ControlPath=~/.ssh/cm-%C"
        -o ControlPersist=10m
      )
      # Non-interactive calls (ps/status/info fan-out from TUI or scripts) must
      # never prompt — BatchMode=yes makes SSH fail silently instead of writing
      # a password dialog to /dev/tty (which would bleed into ratatui TUI output).
      [[ -z "$interactive" ]] && ssh_opts+=(-o BatchMode=yes)
      # Agent forwarding for envs in DEV_SSH_AGENT (set in local.zsh). The agent
      # is selected by $SSH_AUTH_SOCK (1Password; exported in loadConfig or by the
      # macOS login hook) — we never pin IdentityAgent here, matching
      # home/common/cli/ssh.nix. ForwardAgent=yes forwards that same agent so
      # onward `ssh` from the remote reuses the local 1Password keys.
      local _agent_e
      for _agent_e in "''${DEV_SSH_AGENT[@]:-}"; do
        if [[ "$_agent_e" == "$env_name" ]]; then
          ssh_opts+=(-o ForwardAgent=yes)
          break
        fi
      done
      [[ -n "$proxy" ]] && ssh_opts+=(-o "ProxyCommand=$proxy")
      if [[ "$shell" == "pwsh" ]]; then
        local ps_cmd=""
        [[ -n "$rp" ]] && ps_cmd="Set-Location '$rp'; "
        if [[ -n "$interactive" ]]; then
          ssh "''${ssh_opts[@]}" "$flag" "$ssh_host" "''${ps_cmd}pwsh -NoLogo"
        else
          ssh "''${ssh_opts[@]}" "$flag" "$ssh_host" "pwsh -NoLogo -NonInteractive -Command \"''${ps_cmd}''${cmd}\""
        fi
      elif [[ "$shell" == "nu" ]]; then
        # Nushell (e.g. on Windows). Invoke nu by name so the remote default
        # OpenSSH shell (cmd/pwsh/nu) only needs nu on PATH. Outer double quotes
        # survive cmd/pwsh parsing; inner single quotes are nu raw strings
        # (Windows backslash paths stay literal). -e runs then drops to a REPL.
        if [[ -n "$interactive" && -z "$cmd" ]]; then
          if [[ -n "$rp" ]]; then
            ssh "''${ssh_opts[@]}" "$flag" "$ssh_host" "nu -e \"cd '$rp'\""
          else
            ssh "''${ssh_opts[@]}" "$flag" "$ssh_host" "nu"
          fi
        else
          local nu_body="$cmd"
          [[ -n "$rp" ]] && nu_body="cd '$rp'; $cmd"
          ssh "''${ssh_opts[@]}" "$flag" "$ssh_host" "nu -c \"$nu_body\""
        fi
      else
        local sh_cmd=""
        [[ -n "$rp" ]] && sh_cmd="cd $(printf '%q' "$rp") && "
        if [[ -n "$interactive" && -z "$cmd" ]]; then
          sh_cmd+="exec env ZELLIJ=1 ''${shell:-bash} -l"
        else
          sh_cmd+="$cmd"
        fi
        ssh "''${ssh_opts[@]}" "$flag" "$ssh_host" "$sh_cmd"
      fi
    }

    # Resolve name to (env, rp) and run.
    # Accepts: DEV_ENVS name (no cd), DEV_REMOTE project name, DEV_LOCAL project name.
    _dev_resolve_and_run() {
      local name="$1" cmd="''${2:-}" interactive="''${3:-}"
      local env_name rp lp
      if lp=$(_local_get_path "$name" 2>/dev/null); then
        if [[ -n "$interactive" && -z "$cmd" ]]; then
          cd "$lp" && exec "''${SHELL:-bash}"
        elif [[ -n "$interactive" ]]; then
          cd "$lp" && exec bash -c "$cmd"
        else
          cd "$lp" && bash -c "$cmd"
        fi
      elif env_name=$(_remote_get_env "$name" 2>/dev/null); then
        rp=$(_remote_get_path "$name")
        _dev_exec_on_env "$env_name" "$rp" "$cmd" "$interactive"
      elif _env_exists "$name"; then
        _dev_exec_on_env "$name" "" "$cmd" "$interactive"
      else
        echo "dev: unknown name '$name'" >&2
        echo "  projects: $(_dev_list_projects | tr '\n' ' ')" >&2
        echo "  envs:     $(_dev_list_envs | tr '\n' ' ')" >&2
        return 1
      fi
    }

    # Generic agent launcher: _dev_agent <tool> <project> [flags...]
    # Resolves project (local or remote), ensures a TTY via Zellij if needed,
    # then exec's <tool> in the project directory.
    _dev_agent() {
      local tool="$1" name="$2"
      shift 2
      [[ -z "$name" ]] && { echo "Usage: dev $tool <project> [flags...]" >&2; return 1; }
      local _lp="" _env="" _rp=""
      if _lp=$(_local_get_path "$name" 2>/dev/null); then
        :
      elif _env=$(_remote_get_env "$name" 2>/dev/null); then
        _rp=$(_remote_get_path "$name")
      else
        echo "dev: unknown project '$name'" >&2
        echo "  projects: $(_dev_list_projects | tr '\n' ' ')" >&2
        return 1
      fi
      if [[ ! -t 0 ]]; then
        if [[ -n "''${ZELLIJ:-}" ]]; then
          exec zellij run --name "$tool:$name" -- "$0" "$tool" "$name" "$@"
        else
          echo "dev $tool: no TTY and not inside Zellij" >&2
          echo "  In a terminal:       dev $tool $name" >&2
          echo "  In a Zellij session: zellij run -- dev $tool $name" >&2
          return 1
        fi
      fi
      if [[ -n "$_lp" ]]; then
        cd "$_lp" && exec "$tool" "$@"
      else
        _dev_exec_on_env "$_env" "$_rp" "exec $tool $(printf '%q ' "$@")" interactive
      fi
    }

    # Fast, side-effect-free target summary for fzf --preview. Accepts a raw fzf
    # line ("foo", "project foo", or "env  foo") — the name is its last token.
    # Local git is cheap; remote shows static fields only (no SSH per keystroke).
    _dev_preview() {
      local raw="''${1:-}" name lp env_name rp ssh_host proxy shell os
      name="''${raw##* }"
      [[ -z "$name" ]] && return 0
      if lp=$(_local_get_path "$name" 2>/dev/null); then
        echo "TYPE   local project"
        echo "PATH   $lp"
        if git -C "$lp" rev-parse --git-dir >/dev/null 2>&1; then
          echo "BRANCH $(git -C "$lp" branch --show-current 2>/dev/null)"
          echo "HEAD   $(git -C "$lp" log --oneline -1 2>/dev/null)"
          echo "DIRTY  $(git -C "$lp" status --short 2>/dev/null | wc -l | tr -d ' ') file(s)"
        else
          echo "GIT    n/a"
        fi
      elif env_name=$(_remote_get_env "$name" 2>/dev/null); then
        rp=$(_remote_get_path "$name")
        ssh_host=$(_env_get_host "$env_name")
        shell=$(_env_get_shell "$env_name")
        os=$(_env_get_os "$env_name" "$rp")
        echo "TYPE   remote project"
        echo "ENV    $env_name"
        echo "HOST   $ssh_host"
        echo "SHELL  $shell"
        echo "OS     $os"
        echo "PATH   $rp"
        echo ""
        echo "(git status: dev info $name)"
      elif _env_exists "$name"; then
        ssh_host=$(_env_get_host "$name")
        proxy=$(_env_get_proxy "$name")
        shell=$(_env_get_shell "$name")
        os=$(_env_get_os "$name")
        echo "TYPE   env"
        echo "HOST   $ssh_host"
        echo "PROXY  ''${proxy:--}"
        echo "SHELL  $shell"
        echo "OS     $os"
      fi
    }

    _dev_info() {
      local name="$1" lp env_name rp ssh_host proxy shell os
      [[ -z "$name" ]] && name=$(_dev_select_any) || true
      [[ -z "$name" ]] && return 1

      if lp=$(_local_get_path "$name" 2>/dev/null); then
        echo "NAME    $name"
        echo "TYPE    local project"
        echo "PATH    $lp"
        if [[ -d "$lp/.git" || -n "$(git -C "$lp" rev-parse --show-toplevel 2>/dev/null)" ]]; then
          echo "GIT"
          git -C "$lp" log --oneline -1 2>/dev/null | sed 's/^/  head    /'
          git -C "$lp" branch --show-current 2>/dev/null | sed 's/^/  branch  /'
          local dirty
          dirty=$(git -C "$lp" status --short 2>/dev/null | wc -l | tr -d ' ')
          echo "  changes $dirty"
        else
          echo "GIT     n/a"
        fi
      elif env_name=$(_remote_get_env "$name" 2>/dev/null); then
        rp=$(_remote_get_path "$name")
        ssh_host=$(_env_get_host "$env_name")
        proxy=$(_env_get_proxy "$env_name")
        shell=$(_env_get_shell "$env_name")
        os=$(_env_get_os "$env_name" "$rp")
        echo "NAME    $name"
        echo "TYPE    remote project"
        echo "ENV     $env_name"
        echo "HOST    $ssh_host"
        echo "PROXY   ''${proxy:--}"
        echo "SHELL   $shell"
        echo "OS      $os"
        echo "PATH    $rp"
        echo "GIT"
        if rg=$(_dev_remote_git_summary "$env_name" "$rp" 2>/dev/null); then
          while IFS= read -r _l; do
            case "$_l" in
              H:*) echo "  head    ''${_l#H:}" ;;
              B:*) echo "  branch  ''${_l#B:}" ;;
              C:*) echo "  changes ''${_l#C:}" ;;
            esac
          done <<< "$rg"
        else
          echo "  unreachable"
        fi
      elif _env_exists "$name"; then
        ssh_host=$(_env_get_host "$name")
        proxy=$(_env_get_proxy "$name")
        shell=$(_env_get_shell "$name")
        os=$(_env_get_os "$name")
        echo "NAME    $name"
        echo "TYPE    env"
        echo "HOST    $ssh_host"
        echo "PROXY   ''${proxy:--}"
        echo "SHELL   $shell"
        echo "OS      $os"
      else
        echo "dev: unknown name '$name'" >&2
        echo "  projects: $(_dev_list_projects | tr '\n' ' ')" >&2
        echo "  envs:     $(_dev_list_envs | tr '\n' ' ')" >&2
        return 1
      fi
    }

    _dev_code() {
      local name="$1" lp env_name rp ssh_host shell
      [[ -z "$name" || "$name" == "-" ]] && name=$(_dev_select_project)
      [[ -z "$name" ]] && return 1
      command -v code >/dev/null 2>&1 || { echo "dev code: VS Code 'code' command not found" >&2; return 1; }

      if lp=$(_local_get_path "$name" 2>/dev/null); then
        [[ -d "$lp" ]] || { echo "dev code: local path does not exist: $lp" >&2; return 1; }
        exec code "$lp"
      elif env_name=$(_remote_get_env "$name" 2>/dev/null); then
        rp=$(_remote_get_path "$name")
        ssh_host=$(_env_get_host "$env_name")
        shell=$(_env_get_shell "$env_name")
        if [[ "$shell" == "pwsh" ]]; then
          echo "dev code: pwsh remotes are not supported yet" >&2
          return 1
        fi
        exec code --remote "ssh-remote+$ssh_host" "$rp"
      else
        echo "dev code: unknown project '$name'" >&2
        echo "  projects: $(_dev_list_projects | tr '\n' ' ')" >&2
        return 1
      fi
    }

    _dev_doctor() {
      local connect="" failures=0 warnings=0 names=()
      local entry n host proxy shell os path env rp name lp env_name
      while [[ $# -gt 0 ]]; do
        case "$1" in
          --connect) connect=1; shift ;;
          *) names+=("$1"); shift ;;
        esac
      done

      _ok() { printf "ok      %s\n" "$*"; }
      _warn() { printf "warn    %s\n" "$*"; warnings=$((warnings + 1)); }
      _fail() { printf "fail    %s\n" "$*"; failures=$((failures + 1)); }

      echo "TOOLS"
      for tool in ssh git; do
        command -v "$tool" >/dev/null 2>&1 && _ok "$tool" || _fail "$tool not found"
      done
      command -v fzf >/dev/null 2>&1 && _ok "fzf" || _warn "fzf not found; omitted-name selection is unavailable"
      command -v code >/dev/null 2>&1 && _ok "code" || _warn "VS Code 'code' command not found"

      echo ""
      echo "CONFIG"
      [[ ''${#DEV_ENVS[@]} -gt 0 ]] && _ok "DEV_ENVS entries: ''${#DEV_ENVS[@]}" || _warn "DEV_ENVS is empty"
      [[ ''${#DEV_LOCAL[@]} -gt 0 ]] && _ok "DEV_LOCAL entries: ''${#DEV_LOCAL[@]}" || _warn "DEV_LOCAL is empty"
      [[ ''${#DEV_REMOTE[@]} -gt 0 ]] && _ok "DEV_REMOTE entries: ''${#DEV_REMOTE[@]}" || _warn "DEV_REMOTE is empty"

      for entry in "''${DEV_ENVS[@]:-}"; do
        IFS='|' read -r n host proxy shell os <<< "$entry"
        [[ -n "$n" && -n "$host" ]] && _ok "env $n -> $host" || _fail "bad DEV_ENVS entry: $entry"
        case "''${shell:-bash}" in bash|zsh|pwsh|nu) : ;; *) _warn "env $n has unknown shell '$shell'" ;; esac
        case "''${os:-}" in ""|linux|macos|windows) : ;; *) _warn "env $n has unknown os '$os'" ;; esac
      done
      for entry in "''${DEV_LOCAL[@]:-}"; do
        IFS='|' read -r n path <<< "$entry"
        if [[ -z "$n" || -z "$path" ]]; then
          _fail "bad DEV_LOCAL entry: $entry"
        elif [[ -d "$path" ]]; then
          _ok "local $n path exists"
        else
          _fail "local $n path missing: $path"
        fi
      done
      for entry in "''${DEV_REMOTE[@]:-}"; do
        IFS='|' read -r n env rp <<< "$entry"
        if [[ -z "$n" || -z "$env" || -z "$rp" ]]; then
          _fail "bad DEV_REMOTE entry: $entry"
        elif _env_exists "$env"; then
          _ok "remote $n uses env $env"
        else
          _fail "remote $n references unknown env $env"
        fi
      done

      if [[ -n "''${DEV_SSH_AGENT[*]:-}" ]]; then
        local _sock="''${DEV_SSH_AGENT_SOCK:-$HOME/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock}"
        [[ -S "$_sock" ]] && _ok "1Password SSH agent socket present" \
          || _warn "1Password SSH agent socket missing: $_sock"
        local _ae
        for _ae in "''${DEV_SSH_AGENT[@]}"; do
          _env_exists "$_ae" && _ok "ssh-agent forward for env $_ae" \
            || _fail "DEV_SSH_AGENT references unknown env $_ae"
        done
      fi

      if [[ -n "$connect" ]]; then
        echo ""
        echo "CONNECTIVITY"
        if [[ ''${#names[@]} -eq 0 ]]; then
          while IFS= read -r n; do names+=("$n"); done < <(_dev_list_envs)
          while IFS= read -r n; do names+=("$n"); done < <(_dev_list_projects)
        fi
        for name in "''${names[@]}"; do
          if lp=$(_local_get_path "$name" 2>/dev/null); then
            [[ -d "$lp" ]] && _ok "$name local path reachable" || _fail "$name local path missing"
          elif env_name=$(_remote_get_env "$name" 2>/dev/null); then
            rp=$(_remote_get_path "$name")
            shell=$(_env_get_shell "$env_name")
            if [[ "$shell" == "pwsh" ]]; then
              _dev_exec_on_env "$env_name" "" "Test-Path '$rp'" 2>/dev/null | grep -q True && _ok "$name remote path reachable" || _fail "$name remote path unreachable"
            else
              _dev_exec_on_env "$env_name" "" "test -d $(printf '%q' "$rp")" 2>/dev/null && _ok "$name remote path reachable" || _fail "$name remote path unreachable"
            fi
          elif _env_exists "$name"; then
            _dev_exec_on_env "$name" "" "true" 2>/dev/null && _ok "$name ssh reachable" || _fail "$name ssh unreachable"
          else
            _fail "unknown name for connectivity check: $name"
          fi
        done
      else
        echo ""
        echo "CONNECTIVITY"
        _warn "skipped; run 'dev doctor --connect' to check SSH and remote paths"
      fi

      echo ""
      printf "SUMMARY failures=%d warnings=%d\n" "$failures" "$warnings"
      [[ "$failures" -eq 0 ]]
    }

    # ---------------------------------------------------------------------------
    # Machine-readable surface (L0): `dev … --json`.
    # Rule: never printf JSON by hand — build every object with `${jq} -n` so
    # paths/branches/messages with spaces or quotes are encoded correctly.
    # The human output of each subcommand is left byte-for-byte unchanged; a
    # `[[ -n "$JSON" ]]` guard routes to a separate builder.
    # ---------------------------------------------------------------------------

    # Split argv into a JSON flag + positional DEV_ARGS[]. Call at the top of a
    # branch: `_dev_take_flags "$@"; set -- "''${DEV_ARGS[@]}"`.
    _dev_take_flags() {
      JSON=""; DEV_ARGS=()
      local _a
      for _a in "$@"; do
        case "$_a" in
          --json) JSON=1 ;;
          *) DEV_ARGS+=("$_a") ;;
        esac
      done
    }

    _dev_target_kind() {
      local name="$1"
      if   _local_get_path "$name" >/dev/null 2>&1; then echo local-project
      elif _remote_get_env "$name" >/dev/null 2>&1; then echo remote-project
      elif _env_exists     "$name";                 then echo env
      else return 1; fi
    }

    # Flat array of every project + env. The single source of truth that
    # `dev targets --json` and `dev ls --json` both derive from.
    _dev_targets_json() {
      {
        local entry n host proxy shell os path env rp _host _shell _os
        for entry in "''${DEV_LOCAL[@]:-}"; do
          IFS='|' read -r n path <<< "$entry"; [[ -z "$n" ]] && continue
          ${jq} -n --arg name "$n" --arg path "$path" \
            '{name:$name,kind:"local-project",env:null,host:null,shell:null,os:null,proxy:null,path:$path}'
        done
        for entry in "''${DEV_REMOTE[@]:-}"; do
          IFS='|' read -r n env rp <<< "$entry"; [[ -z "$n" ]] && continue
          _host=$(_env_get_host "$env" 2>/dev/null); _shell=$(_env_get_shell "$env" 2>/dev/null)
          _os=$(_env_get_os "$env" "$rp" 2>/dev/null)
          ${jq} -n --arg name "$n" --arg env "$env" --arg host "$_host" --arg shell "$_shell" --arg os "$_os" --arg path "$rp" \
            '{name:$name,kind:"remote-project",env:$env,host:$host,shell:$shell,os:$os,proxy:null,path:$path}'
        done
        for entry in "''${DEV_ENVS[@]:-}"; do
          IFS='|' read -r n host proxy shell os <<< "$entry"; [[ -z "$n" ]] && continue
          _os=$(_env_get_os "$n" 2>/dev/null)
          ${jq} -n --arg name "$n" --arg host "$host" --arg proxy "$proxy" --arg shell "''${shell:-bash}" --arg os "$_os" \
            '{name:$name,kind:"env",env:$name,host:$host,shell:$shell,os:$os,proxy:$proxy,path:null}'
        done
      } | ${jq} -s '.'
    }

    # Backend registry — the ONE place agent backends are enumerated.
    # fields: name, interactive, dispatchable, review, ps_detect, attach, bg_sub
    #   ps_detect : agents-json (claude) | pgrep | pgrep-f
    #   bg_sub    : token after tool name for nohup dispatch; empty = claude --bg
    #   attach    : strategy key used by _dev_attach
    _dev_tools_json() {
      local rows=(
        "claude|1|1|1|agents-json|claude-resume|"
        "codex|1|1|1|pgrep|codex-resume|exec"
        "opencode|1|1|1|pgrep|opencode-continue|run"
        "agy|1|1|1|pgrep|agy-fresh|-p"
      )
      local r n it dp rv pd at bs
      for r in "''${rows[@]}"; do
        IFS='|' read -r n it dp rv pd at bs <<< "$r"
        ${jq} -n --arg n "$n" --argjson it "$it" --argjson dp "$dp" --argjson rv "$rv" \
          --arg pd "$pd" --arg at "$at" --arg bs "$bs" \
          '{name:$n,interactive:($it==1),dispatchable:($dp==1),review:($rv==1),ps_detect:$pd,attach:$at,bg_sub:$bs}'
      done | ${jq} -s '.'
    }

    # Accessor: get a single field for a named tool.  Returns empty string if
    # the tool is not in the registry.
    _dev_tool_field() {
      _dev_tools_json | ${jq} -r --arg n "$1" --arg f "$2" \
        '.[]|select(.name==$n)|.[$f] // ""'
    }

    _ps_single_quote() {
      printf '%s' "$1"
    }

    _ps_encoded_command() {
      printf '%s' "$1" | iconv -f UTF-8 -t UTF-16LE | base64 | tr -d '\n'
    }

    _dev_exec_windows_ps() {
      local env_name="$1" ps_script="$2" shell enc
      shell=$(_env_get_shell "$env_name")
      enc=$(_ps_encoded_command "$ps_script") || return 1
      if [[ "$shell" == "nu" ]]; then
        _dev_exec_on_env "$env_name" "" "if ((which pwsh | length) > 0) { pwsh -NoLogo -NoProfile -NonInteractive -EncodedCommand $enc } else { powershell -NoLogo -NoProfile -NonInteractive -EncodedCommand $enc }"
      else
        _dev_exec_on_env "$env_name" "" "if (Get-Command pwsh -ErrorAction SilentlyContinue) { pwsh -NoLogo -NoProfile -NonInteractive -EncodedCommand $enc } else { powershell -NoLogo -NoProfile -NonInteractive -EncodedCommand $enc }"
      fi
    }

    _dev_remote_git_summary() {
      local env_name="$1" rp="$2" os qrp ps
      os=$(_env_get_os "$env_name" "$rp")
      if [[ "$os" == "windows" ]]; then
        qrp=$(_ps_single_quote "$rp")
        ps="
          \$ErrorActionPreference = 'SilentlyContinue'
          Set-Location -LiteralPath '$qrp'
          Write-Output ('B:' + (& git branch --show-current))
          Write-Output ('H:' + (& git log --oneline -1))
          \$changes = (& git status --short | Measure-Object -Line).Lines
          Write-Output ('C:' + \$changes)
        "
        _dev_exec_windows_ps "$env_name" "$ps"
      else
        _dev_exec_on_env "$env_name" "$rp" "echo \"B:\$(git branch --show-current 2>/dev/null)\"; echo \"H:\$(git log --oneline -1 2>/dev/null)\"; echo \"C:\$(git status --short 2>/dev/null | wc -l | tr -d ' ')\""
      fi
    }

    _dev_remote_git_human() {
      local env_name="$1" rp="$2" os qrp ps
      os=$(_env_get_os "$env_name" "$rp")
      if [[ "$os" == "windows" ]]; then
        qrp=$(_ps_single_quote "$rp")
        ps="
          \$ErrorActionPreference = 'SilentlyContinue'
          Set-Location -LiteralPath '$qrp'
          & git log --oneline -2
          & git status --short
        "
        _dev_exec_windows_ps "$env_name" "$ps"
      else
        _dev_exec_on_env "$env_name" "$rp" "git log --oneline -2 && git status --short"
      fi
    }

    _dev_windows_claude_agents_json() {
      local env_name="$1" rp="$2" qrp ps
      qrp=$(_ps_single_quote "$rp")
      ps="
        \$ErrorActionPreference = 'SilentlyContinue'
        \$json = '[]'
        if (Get-Command claude -ErrorAction SilentlyContinue) {
          \$raw = & claude agents --json --cwd '$qrp' 2>\$null
          if (\$LASTEXITCODE -eq 0 -and \$raw) { \$json = (\$raw -join [Environment]::NewLine) }
        }
        [Console]::Out.WriteLine(\$json)
      "
      _dev_exec_windows_ps "$env_name" "$ps"
    }

    _dev_windows_process_rows() {
      local env_name="$1" rp="$2" qrp ps
      qrp=$(_ps_single_quote "$rp")
      ps="
        \$ErrorActionPreference = 'SilentlyContinue'
        \$base = '$qrp'
        \$needle = \$base.Replace('\', '/')
        \$tools = @('codex','opencode','agy')
        Get-CimInstance Win32_Process | ForEach-Object {
          \$tool = [IO.Path]::GetFileNameWithoutExtension([string]\$_.Name)
          if (\$tools -contains \$tool) {
            \$cmd = [string]\$_.CommandLine
            \$norm = \$cmd.Replace('\', '/')
            if (\$norm.Contains(\$needle)) {
              Write-Output (('{0} {1} {2}' -f \$tool, \$_.ProcessId, \$base))
            }
          }
        }
      "
      _dev_exec_windows_ps "$env_name" "$ps"
    }

    # dev ls --json — grouped to mirror the human sections.
    _dev_ls_json() {
      _dev_targets_json | ${jq} '{
        envs:   [.[] | select(.kind=="env")            | {name,host,proxy,shell,os}],
        local:  [.[] | select(.kind=="local-project")  | {name,path}],
        remote: [.[] | select(.kind=="remote-project") | {name,env,host,shell,os,path}]
      }'
    }

    # dev info --json — single object (re-resolves; does not touch the human path).
    _dev_info_json() {
      local name="$1" lp env_name rp ssh_host proxy shell os
      local branch="" head="" changes="" gitok=false rg _l
      if lp=$(_local_get_path "$name" 2>/dev/null); then
        if [[ -d "$lp/.git" || -n "$(git -C "$lp" rev-parse --show-toplevel 2>/dev/null)" ]]; then
          gitok=true
          head=$(git -C "$lp" log --oneline -1 2>/dev/null)
          branch=$(git -C "$lp" branch --show-current 2>/dev/null)
          changes=$(git -C "$lp" status --short 2>/dev/null | wc -l | tr -d ' ')
        fi
        ${jq} -n --arg name "$name" --arg path "$lp" --argjson gitok "$gitok" \
          --arg branch "$branch" --arg head "$head" --arg changes "''${changes:-0}" \
          '{name:$name,kind:"local-project",env:null,host:null,proxy:null,shell:null,os:null,path:$path,
            git:(if $gitok then {branch:$branch,head:$head,changes:($changes|tonumber)} else null end)}'
      elif env_name=$(_remote_get_env "$name" 2>/dev/null); then
        rp=$(_remote_get_path "$name"); ssh_host=$(_env_get_host "$env_name")
        proxy=$(_env_get_proxy "$env_name"); shell=$(_env_get_shell "$env_name")
        os=$(_env_get_os "$env_name" "$rp")
        rg=$(_dev_remote_git_summary "$env_name" "$rp" 2>/dev/null) && {
          gitok=true
          while IFS= read -r _l; do
            case "$_l" in B:*) branch="''${_l#B:}" ;; H:*) head="''${_l#H:}" ;; C:*) changes="''${_l#C:}" ;; esac
          done <<< "$rg"
        }
        ${jq} -n --arg name "$name" --arg env "$env_name" --arg host "$ssh_host" --arg proxy "$proxy" \
          --arg shell "$shell" --arg os "$os" --arg path "$rp" --argjson gitok "$gitok" \
          --arg branch "$branch" --arg head "$head" --arg changes "''${changes:-0}" \
          '{name:$name,kind:"remote-project",env:$env,host:$host,proxy:$proxy,shell:$shell,os:$os,path:$path,
            git:(if $gitok then {branch:$branch,head:$head,changes:($changes|tonumber)} else null end)}'
      elif _env_exists "$name"; then
        ssh_host=$(_env_get_host "$name"); proxy=$(_env_get_proxy "$name"); shell=$(_env_get_shell "$name"); os=$(_env_get_os "$name")
        ${jq} -n --arg name "$name" --arg host "$ssh_host" --arg proxy "$proxy" --arg shell "$shell" --arg os "$os" \
          '{name:$name,kind:"env",env:$name,host:$host,proxy:$proxy,shell:$shell,os:$os,path:null,git:null}'
      else
        echo "dev: unknown name '$name'" >&2; return 1
      fi
    }

    # dev status --json — array; parallel like the human path, one object/target.
    _dev_status_json() {
      local names=("$@") tmp n lp env_name rp
      tmp=$(mktemp -d) || return 1
      for n in "''${names[@]}"; do
        (
          local br="" hd="" ch="" rg _l
          if lp=$(_local_get_path "$n" 2>/dev/null); then
            if [[ -d "$lp/.git" || -n "$(git -C "$lp" rev-parse --show-toplevel 2>/dev/null)" ]]; then
              br=$(git -C "$lp" branch --show-current 2>/dev/null)
              hd=$(git -C "$lp" log --oneline -1 2>/dev/null)
              ch=$(git -C "$lp" status --short 2>/dev/null | wc -l | tr -d ' ')
              ${jq} -n -c --arg t "$n" --arg br "$br" --arg hd "$hd" --argjson ch "''${ch:-0}" \
                '{target:$t,kind:"local",ok:true,branch:$br,head:$hd,changes:$ch}' > "$tmp/$n"
            else
              ${jq} -n -c --arg t "$n" '{target:$t,kind:"local",ok:true,branch:null,head:null,changes:null}' > "$tmp/$n"
            fi
          elif env_name=$(_remote_get_env "$n" 2>/dev/null); then
            rp=$(_remote_get_path "$n")
            if rg=$(_dev_remote_git_summary "$env_name" "$rp" 2>/dev/null); then
              while IFS= read -r _l; do
                case "$_l" in B:*) br="''${_l#B:}" ;; H:*) hd="''${_l#H:}" ;; C:*) ch="''${_l#C:}" ;; esac
              done <<< "$rg"
              ${jq} -n -c --arg t "$n" --arg br "$br" --arg hd "$hd" --argjson ch "''${ch:-0}" \
                '{target:$t,kind:"remote",ok:true,branch:$br,head:$hd,changes:$ch}' > "$tmp/$n"
            else
              ${jq} -n -c --arg t "$n" '{target:$t,kind:"remote",ok:false,branch:null,head:null,changes:null}' > "$tmp/$n"
            fi
          fi
        ) &
      done
      wait
      for n in "''${names[@]}"; do cat "$tmp/$n" 2>/dev/null; done | ${jq} -s '.'
      rm -rf "$tmp"
    }

    # Fan-out (L1): run one command across many targets concurrently. Reuses the
    # status/ps idiom (per-target subshell → tmpfile → wait). json="" → human
    # blocks + SUMMARY; json=1 → [{target,ok,exit,stdout,stderr}]. Returns
    # nonzero if any target failed (useful for an orchestrating agent).
    _dev_run_fanout() {
      local json="$1" qcmd="$2"; shift 2
      local targets=("$@") t tmp env rc fail=0 ok=0
      tmp=$(mktemp -d) || return 1
      # Pre-warm one ControlMaster per distinct remote env so the concurrent SSH
      # below reuse the master instead of racing to create it.
      local -a warmed=()
      for t in "''${targets[@]}"; do
        env=$(_remote_get_env "$t" 2>/dev/null) || continue
        [[ " ''${warmed[*]:-} " == *" $env "* ]] && continue
        warmed+=("$env"); _dev_exec_on_env "$env" "" true >/dev/null 2>&1 &
      done
      wait
      for t in "''${targets[@]}"; do
        ( out=$(_dev_resolve_and_run "$t" "$qcmd" 2>"$tmp/$t.err"); rc=$?
          printf '%s' "$out" > "$tmp/$t.out"; echo "$rc" > "$tmp/$t.rc" ) &
      done
      wait
      if [[ -n "$json" ]]; then
        for t in "''${targets[@]}"; do
          rc=$(cat "$tmp/$t.rc" 2>/dev/null || echo 1)
          [[ "$rc" == 0 ]] && ok=$((ok + 1)) || fail=$((fail + 1))
          ${jq} -n -c --arg target "$t" --argjson exit "''${rc:-1}" \
            --arg stdout "$(cat "$tmp/$t.out" 2>/dev/null)" \
            --arg stderr "$(cat "$tmp/$t.err" 2>/dev/null)" \
            '{target:$target,ok:($exit==0),exit:$exit,stdout:$stdout,stderr:$stderr}'
        done | ${jq} -s '.'
      else
        for t in "''${targets[@]}"; do
          rc=$(cat "$tmp/$t.rc" 2>/dev/null || echo 1)
          [[ "$rc" == 0 ]] && ok=$((ok + 1)) || fail=$((fail + 1))
          echo "=== $t (exit $rc) ==="
          [[ -s "$tmp/$t.out" ]] && { cat "$tmp/$t.out"; echo; }
          [[ -s "$tmp/$t.err" ]] && { echo "--- stderr ($t) ---"; cat "$tmp/$t.err"; }
        done
        printf 'SUMMARY ok=%d fail=%d\n' "$ok" "$fail"
      fi
      rm -rf "$tmp"
      [[ "$fail" -eq 0 ]]
    }

    # ============ L2: dispatch & supervise ============
    # Per-target run registry under $HOME/.dev/runs/<id>.{meta,log}. ids are
    # <tool>-<project>-<epoch>. Meta (compact JSON) is built locally and shipped
    # to the target. claude uses its native --bg (tracked by `claude agents`);
    # codex/opencode are detached with setsid (own session → survive SSH close)
    # and logged. Target-side $HOME must stay literal until it runs there, so it
    # is written as \$HOME in this Nix source.

    # Resolve a project → globals R_LOC(local|remote) R_ENV R_PATH(base path).
    _dev_project_resolve() {
      local name="$1" lp env_name
      R_LOC=""; R_ENV=""; R_PATH=""
      if lp=$(_local_get_path "$name" 2>/dev/null); then
        R_LOC=local; R_PATH="$lp"
      elif env_name=$(_remote_get_env "$name" 2>/dev/null); then
        R_LOC=remote; R_ENV="$env_name"; R_PATH=$(_remote_get_path "$name")
      else
        return 1
      fi
    }

    # Run a command string on the resolved target at cwd=$1 (worktree-aware).
    _dev_run_at() {
      local cwd="$1" cmd="$2"
      if [[ "$R_LOC" == local ]]; then ( cd "$cwd" 2>/dev/null && bash -c "$cmd" )
      else _dev_exec_on_env "$R_ENV" "$cwd" "$cmd"; fi
    }

    # Registry IO — $1 is a bare filename under $HOME/.dev/runs (target-side).
    _dev_put_run() {
      if [[ "$R_LOC" == local ]]; then mkdir -p "$HOME/.dev/runs"; cat > "$HOME/.dev/runs/$1"
      else _dev_exec_on_env "$R_ENV" "" "mkdir -p \"\$HOME/.dev/runs\" && cat > \"\$HOME/.dev/runs/$1\""; fi
    }

    # Newest run meta (compact JSON) for a project (sets R_* first), or "".
    _dev_run_meta() {
      local ref="$1" q
      _dev_project_resolve "$ref" 2>/dev/null || return 1
      q=$(printf '%q' "\"project\":\"$ref\"")
      _dev_run_at "$R_PATH" "{ ls -t \"\$HOME/.dev/runs\" 2>/dev/null | grep -F '.meta' | while IFS= read -r f; do printf '%s\n' \"\$HOME/.dev/runs/\$f\"; done; } | while IFS= read -r p; do grep -q $q \"\$p\" && { cat \"\$p\"; break; }; done"
    }

    # Ensure a worktree for <branch> off <base> on the target; echoes its path.
    # Sibling .dev-worktrees/<repo>-<branch> — never auto-removed.
    _dev_worktree_ensure() {
      local base="$1" branch="$2" repo san wt qbase qwt qbr
      repo=$(basename "$base"); san=$(printf '%s' "$branch" | tr '/ ' '--')
      wt="$(dirname "$base")/.dev-worktrees/$repo-$san"
      qbase=$(printf '%q' "$base"); qwt=$(printf '%q' "$wt"); qbr=$(printf '%q' "$branch")
      _dev_run_at "$base" "if [ -d $qwt ]; then :; else git -C $qbase worktree add -b $qbr $qwt 2>/dev/null || git -C $qbase worktree add $qwt $qbr 2>/dev/null; fi" >/dev/null 2>&1
      printf '%s\n' "$wt"
    }

    _dev_dispatch() {
      local tool=claude worktree="" json="" project="" task="" model="" effort="" sandbox=""
      while [[ $# -gt 0 ]]; do
        case "$1" in
          --tool)     tool="$2";    shift 2 ;;
          --worktree) worktree="$2"; shift 2 ;;
          --model)    model="$2";   shift 2 ;;
          --effort)   effort="$2";  shift 2 ;;
          --sandbox)  sandbox="$2"; shift 2 ;;
          --json)     json=1;       shift ;;
          *) if [[ -z "$project" ]]; then project="$1"; else task="''${task:+$task }$1"; fi; shift ;;
        esac
      done
      [[ -z "$project" || -z "$task" ]] && { echo "Usage: dev dispatch <project> [--tool claude|codex|opencode] [--model m] [--effort e] [--sandbox s] [--worktree b] \"<task>\"" >&2; return 1; }
      _dev_project_resolve "$project" || { echo "dev dispatch: unknown project '$project'" >&2; return 1; }
      local cwd="$R_PATH" branch="" wt="" id="$tool-$project-$(date +%s)"
      if [[ -n "$worktree" ]]; then branch="$worktree"; wt=$(_dev_worktree_ensure "$R_PATH" "$worktree"); cwd="$wt"; fi
      local qtask qcwd pid="" session=""
      qtask=$(printf '%q' "$task"); qcwd=$(printf '%q' "$cwd")
      case "$tool" in
        claude)
          # Build optional model/effort flags.
          local mf=""
          [[ -n "$model"  ]] && mf="$mf --model $(printf '%q' "$model")"
          [[ -n "$effort" ]] && mf="$mf --effort $(printf '%q' "$effort")"
          _dev_run_at "$cwd" "mkdir -p \"\$HOME/.dev/runs\"; claude --bg$mf -p $qtask >/dev/null 2>&1 </dev/null || true"
          local cj
          cj=$(_dev_run_at "$cwd" "claude agents --json --cwd $qcwd 2>/dev/null")
          session=$(printf '%s' "$cj" | ${jq} -r 'sort_by(.startedAt)|last|.sessionId // ""' 2>/dev/null)
          pid=$(printf '%s' "$cj" | ${jq} -r 'sort_by(.startedAt)|last|.pid // ""' 2>/dev/null)
          ;;
        *)
          # Registry-driven dispatch for all non-claude backends.
          # bg_sub is the subcommand token (codex=exec, opencode=run, agy=-p, …).
          # The detached pid is recorded so _dev_kill and dev ps can locate it.
          local sub; sub=$(_dev_tool_field "$tool" bg_sub)
          if [[ -z "$(_dev_tool_field "$tool" name)" ]]; then
            echo "dev dispatch: unknown tool '$tool'" >&2; return 1
          fi
          if [[ -z "$sub" ]]; then
            echo "dev dispatch: '$tool' is not background-dispatchable" >&2; return 1
          fi
          # Per-tool extra flags (model/sandbox).  safe: qmodel/qsb are printf '%q'.
          local extra=""
          case "$tool" in
            codex)
              [[ -n "$model"   ]] && extra="$extra --model $(printf '%q' "$model")"
              [[ -n "$sandbox" ]] && extra="$extra --sandbox $(printf '%q' "$sandbox")"
              extra="$extra --ask-for-approval never"
              ;;
            opencode)
              [[ -n "$model" ]] && extra="$extra --model $(printf '%q' "$model")"
              extra="$extra --format json"
              ;;
          esac
          # nohup (not setsid — absent on macOS) detaches from SIGHUP so the
          # agent survives the SSH session / shell exiting; logged to the registry.
          pid=$(_dev_run_at "$cwd" "mkdir -p \"\$HOME/.dev/runs\"; nohup $tool $sub$extra $qtask >\"\$HOME/.dev/runs/$id.log\" 2>&1 </dev/null & echo \$!; disown 2>/dev/null; true")
          ;;
      esac
      local meta
      meta=$(${jq} -n -c --arg id "$id" --arg tool "$tool" --arg project "$project" --arg branch "$branch" \
        --arg wt "$wt" --arg cwd "$cwd" --arg task "$task" --arg pid "$pid" --arg session "$session" \
        --arg log "$id.log" --arg started "$(date -u +%FT%TZ)" \
        '{id:$id,tool:$tool,project:$project,branch:$branch,worktree:$wt,cwd:$cwd,task:$task,pid:$pid,session:$session,log:$log,started:$started}')
      printf '%s\n' "$meta" | _dev_put_run "$id.meta"
      if [[ -n "$json" ]]; then
        printf '%s' "$meta" | ${jq} -c '{id,target:.project,tool,pid,session,branch,worktree,ok:true}'
      else
        echo "dispatched $id on $project (tool=$tool''${pid:+ pid=$pid}''${session:+ session=$session}''${wt:+ worktree=$wt})"
      fi
    }

    _dev_logs() {
      local follow="" json="" ref=""
      while [[ $# -gt 0 ]]; do
        case "$1" in -f|--follow) follow=1; shift ;; --json) json=1; shift ;; *) ref="$1"; shift ;; esac
      done
      [[ -z "$ref" ]] && { echo "Usage: dev logs <project|id> [-f] [--json]" >&2; return 1; }
      # Resolve in THIS shell so R_* are set for the _dev_run_at calls below
      # (a $(_dev_run_meta) subshell would set them only in its own subshell).
      _dev_project_resolve "$ref" || { echo "dev logs: unknown project '$ref'" >&2; return 1; }
      local meta; meta=$(_dev_run_meta "$ref")
      [[ -z "$meta" ]] && { echo "dev logs: no run found for '$ref'" >&2; return 1; }
      local tool log; tool=$(printf '%s' "$meta" | ${jq} -r '.tool'); log=$(printf '%s' "$meta" | ${jq} -r '.log')
      if [[ "$tool" == claude ]]; then
        echo "dev logs: claude logs to its own store — use 'dev attach $ref' or 'dev ps'" >&2; return 1
      fi
      local tailcmd; if [[ -n "$follow" ]]; then tailcmd="tail -n 200 -f"; else tailcmd="tail -n 200"; fi
      if [[ -n "$json" ]]; then
        _dev_run_at "$R_PATH" "$tailcmd \"\$HOME/.dev/runs/$log\" 2>/dev/null" \
          | ${jq} -R -s --arg t "$ref" --arg log "$log" '{target:$t,log:$log,lines:(rtrimstr("\n")|split("\n"))}'
      else
        _dev_run_at "$R_PATH" "$tailcmd \"\$HOME/.dev/runs/$log\" 2>/dev/null"
      fi
    }

    _dev_kill() {
      local json="" ref=""
      while [[ $# -gt 0 ]]; do case "$1" in --json) json=1; shift ;; *) ref="$1"; shift ;; esac; done
      [[ -z "$ref" ]] && { echo "Usage: dev kill <project|id> [--json]" >&2; return 1; }
      _dev_project_resolve "$ref" || { echo "dev kill: unknown project '$ref'" >&2; return 1; }
      local meta; meta=$(_dev_run_meta "$ref")
      [[ -z "$meta" ]] && { echo "dev kill: no run found for '$ref'" >&2; return 1; }
      local tool cwd qcwd killed; tool=$(printf '%s' "$meta" | ${jq} -r '.tool'); cwd=$(printf '%s' "$meta" | ${jq} -r '.cwd')
      qcwd=$(printf '%q' "$cwd")
      if [[ "$tool" == claude ]]; then
        local pids
        pids=$(_dev_run_at "$cwd" "claude agents --json --cwd $qcwd 2>/dev/null" | ${jq} -r '.[]?.pid' 2>/dev/null | tr '\n' ' ')
        [[ -n "''${pids// /}" ]] && _dev_run_at "$cwd" "kill $pids 2>/dev/null; true"
        killed="$pids"
      else
        # native binary → match running procs by cwd (lsof on macOS, /proc on Linux) and TERM them
        killed=$(_dev_run_at "$cwd" "
          for pid in \$(pgrep -x $tool 2>/dev/null); do
            pcwd=\$(lsof -p \$pid -a -d cwd -Fn 2>/dev/null | sed -n 's/^n//p' | head -1)
            [ -z \"\$pcwd\" ] && pcwd=\$(readlink /proc/\$pid/cwd 2>/dev/null)
            case \"\$pcwd\" in $qcwd*) kill \$pid 2>/dev/null && echo \$pid;; esac
          done")
        killed=$(printf '%s' "$killed" | tr '\n' ' ')
      fi
      if [[ -n "$json" ]]; then
        ${jq} -n -c --arg t "$ref" --arg killed "$killed" '{target:$t,killed:($killed|split(" ")|map(select(length>0))),ok:true}'
      else
        echo "killed $ref: ''${killed:-none}"
      fi
    }

    _dev_attach() {
      local ref="$1"
      [[ -z "$ref" || "$ref" == "-" ]] && ref=$(_dev_select_project)
      [[ -z "$ref" ]] && return 1
      local meta; meta=$(_dev_run_meta "$ref")
      if [[ -z "$meta" ]]; then
        # no dispatched run — just open the project's agent (claude) interactively
        _dev_agent claude "$ref"; return
      fi
      local tool session; tool=$(printf '%s' "$meta" | ${jq} -r '.tool'); session=$(printf '%s' "$meta" | ${jq} -r '.session // ""')
      case "$(_dev_tool_field "$tool" attach 2>/dev/null || echo "")" in
        claude-resume)    if [[ -n "$session" ]]; then _dev_agent claude "$ref" --resume "$session"; else _dev_agent claude "$ref"; fi ;;
        codex-resume)     _dev_agent codex "$ref" resume --last ;;
        opencode-continue) _dev_agent opencode "$ref" --continue ;;
        agy-fresh)        _dev_agent agy "$ref" ;;  # agy has no resume API → fresh interactive
        *) echo "dev attach: cannot attach tool '$tool'; try 'dev logs $ref -f'" >&2; return 1 ;;
      esac
    }

    _dev_worktree() {
      local sub="''${1:-}"; shift || true
      case "$sub" in
        list)
          local proj="''${1:-}"; [[ -z "$proj" ]] && { echo "Usage: dev worktree list <project>" >&2; return 1; }
          _dev_project_resolve "$proj" || { echo "dev worktree: unknown project '$proj'" >&2; return 1; }
          _dev_run_at "$R_PATH" "git -C $(printf '%q' "$R_PATH") worktree list"
          ;;
        rm|remove)
          local proj="''${1:-}" branch="''${2:-}"
          [[ -z "$proj" || -z "$branch" ]] && { echo "Usage: dev worktree rm <project> <branch>" >&2; return 1; }
          _dev_project_resolve "$proj" || { echo "dev worktree: unknown project '$proj'" >&2; return 1; }
          local repo san wt; repo=$(basename "$R_PATH"); san=$(printf '%s' "$branch" | tr '/ ' '--')
          wt="$(dirname "$R_PATH")/.dev-worktrees/$repo-$san"
          _dev_run_at "$R_PATH" "git -C $(printf '%q' "$R_PATH") worktree remove $(printf '%q' "$wt")" && echo "removed $wt"
          ;;
        *) echo "Usage: dev worktree <list|rm> <project> [branch]" >&2; return 1 ;;
      esac
    }

    # Telegram push (L3). The token lives in ~/.op-secrets (1Password cache,
    # see home/mac/default.nix) which loadConfig does NOT source, so pull it in
    # here; TELEGRAM_CHAT_ID comes from local.zsh. No-op with a clear error when
    # either is unset.
    _dev_notify() {
      local text="$1"
      [[ -f "$HOME/.op-secrets" ]] && source "$HOME/.op-secrets"
      local token="''${TELEGRAM_BOT_TOKEN:-}" chat="''${TELEGRAM_CHAT_ID:-}"
      if [[ -z "$token" || -z "$chat" ]]; then
        echo "dev notify: TELEGRAM_BOT_TOKEN (op-secrets) or TELEGRAM_CHAT_ID (local.zsh) unset" >&2
        return 1
      fi
      ${curl} -fsS -X POST "https://api.telegram.org/bot$token/sendMessage" \
        --data-urlencode "chat_id=$chat" --data-urlencode "text=$text" >/dev/null
    }

    # ============ L3: harvest & watch ============

    # Worktree-aware path for a project: the dispatched run's worktree if any,
    # else the base path. Sets R_* (in this shell) as a side effect.
    _dev_target_path() {
      local ref="$1" meta wt
      _dev_project_resolve "$ref" || return 1
      meta=$(_dev_run_meta "$ref")
      if [[ -n "$meta" ]]; then
        wt=$(printf '%s' "$meta" | ${jq} -r '.worktree // ""')
        [[ -n "$wt" ]] && { printf '%s' "$wt"; return 0; }
      fi
      printf '%s' "$R_PATH"
    }

    _dev_diff() {
      local stat="" json="" ref=""
      while [[ $# -gt 0 ]]; do
        case "$1" in --stat) stat=1; shift ;; --json) json=1; shift ;; *) ref="$1"; shift ;; esac
      done
      [[ -z "$ref" ]] && { echo "Usage: dev diff <project> [--stat] [--json]" >&2; return 1; }
      _dev_project_resolve "$ref" || { echo "dev diff: unknown project '$ref'" >&2; return 1; }
      local path qpath; path=$(_dev_target_path "$ref")
      qpath=$(printf '%q' "$path")
      if [[ -n "$json" ]]; then
        local branch numstat diff files
        branch=$(_dev_run_at "$path" "git -C $qpath branch --show-current 2>/dev/null")
        numstat=$(_dev_run_at "$path" "git -C $qpath --no-pager diff --numstat 2>/dev/null")
        diff=$(_dev_run_at "$path" "git -C $qpath --no-pager diff 2>/dev/null")
        files=$(printf '%s' "$numstat" | ${jq} -R -s -c 'split("\n")|map(select(length>0)|split("\t")|{path:.[2],added:(.[0]|tonumber? // 0),removed:(.[1]|tonumber? // 0)})')
        ${jq} -n -c --arg t "$ref" --arg path "$path" --arg branch "$branch" \
          --argjson files "''${files:-[]}" --arg diff "$diff" \
          '{target:$t,path:$path,branch:$branch,files:$files,diff:$diff}'
      elif [[ -n "$stat" ]]; then
        _dev_run_at "$path" "git -C $qpath --no-pager diff --stat 2>/dev/null"
      else
        _dev_run_at "$path" "git -C $qpath --no-pager diff 2>/dev/null; git -C $qpath status --short 2>/dev/null"
      fi
    }

    _dev_pr() {
      local title="" base="main" draft="" json="" ref=""
      while [[ $# -gt 0 ]]; do
        case "$1" in
          --title) title="$2"; shift 2 ;; --base) base="$2"; shift 2 ;;
          --draft) draft=1; shift ;; --json) json=1; shift ;; *) ref="$1"; shift ;;
        esac
      done
      [[ -z "$ref" ]] && { echo "Usage: dev pr <project> [--title t] [--base b] [--draft] [--json]" >&2; return 1; }
      command -v gh >/dev/null 2>&1 || { echo "dev pr: gh not found on this Mac" >&2; return 1; }
      _dev_project_resolve "$ref" || { echo "dev pr: unknown project '$ref'" >&2; return 1; }
      local path qpath branch; path=$(_dev_target_path "$ref")
      qpath=$(printf '%q' "$path")
      branch=$(_dev_run_at "$path" "git -C $qpath branch --show-current 2>/dev/null")
      [[ -z "$branch" ]] && { echo "dev pr: no current branch at $path" >&2; return 1; }
      # Push on the target — the forwarded 1Password agent provides the keys.
      _dev_run_at "$path" "git -C $qpath push -u origin $(printf '%q' "$branch") 2>&1" || { echo "dev pr: push failed" >&2; return 1; }
      local remoteurl owner_repo
      remoteurl=$(_dev_run_at "$path" "git -C $qpath remote get-url origin 2>/dev/null")
      owner_repo=$(printf '%s' "$remoteurl" | sed -E 's#^(git@[^:]+:|ssh://git@[^/]+/|https?://[^/]+/)##; s#\.git$##')
      local args=(pr create --repo "$owner_repo" --head "$branch" --base "$base" --fill)
      [[ -n "$title" ]] && args+=(--title "$title")
      [[ -n "$draft" ]] && args+=(--draft)
      local out url; out=$(gh "''${args[@]}" 2>&1) || { echo "$out" >&2; return 1; }
      url=$(printf '%s' "$out" | grep -oE 'https://[^ ]+/pull/[0-9]+' | head -1)
      if [[ -n "$json" ]]; then
        ${jq} -n -c --arg t "$ref" --arg branch "$branch" --arg url "$url" '{target:$t,branch:$branch,url:$url,ok:true}'
      else
        echo "PR: ''${url:-$out}"
      fi
    }

    # ============ review ============
    # Run a code-review agent on a project's uncommitted (or branched) diff.
    # Codex has a native `codex review` command; other tools receive the diff
    # via an agent prompt and use their own tool-use to read `git diff`.
    _dev_review() {
      local tool="" base="" json="" ref=""
      while [[ $# -gt 0 ]]; do
        case "$1" in
          --tool)  tool="$2";  shift 2 ;;
          --base)  base="$2";  shift 2 ;;
          --json)  json=1;     shift ;;
          *)       ref="$1";   shift ;;
        esac
      done
      [[ -z "$ref" ]] && { echo "Usage: dev review <project> [--tool codex|opencode|agy|claude] [--base <ref>] [--json]" >&2; return 1; }
      _dev_project_resolve "$ref" || { echo "dev review: unknown project '$ref'" >&2; return 1; }
      local path qpath; path=$(_dev_target_path "$ref"); qpath=$(printf '%q' "$path")
      # Auto-select: codex first (native review), then opencode, then claude.
      if [[ -z "$tool" ]]; then
        for _t in codex opencode claude; do
          if _dev_run_at "$path" "command -v $_t >/dev/null 2>&1"; then tool="$_t"; break; fi
        done
      fi
      [[ -z "$tool" ]] && { echo "dev review: no supported review tool found" >&2; return 1; }
      local base_arg=""
      [[ -n "$base" ]] && base_arg="--base $(printf '%q' "$base")"
      local review_prompt="Review all uncommitted changes in this project for bugs, security issues, and code quality. Use git diff to inspect what changed. Provide concise, actionable feedback."
      [[ -n "$base" ]] && review_prompt="Review changes since $(printf '%s' "$base") for bugs, security issues, and code quality. Use git diff to inspect. Provide concise, actionable feedback."
      local out rc=0
      case "$tool" in
        codex)
          if [[ -n "$base" ]]; then
            out=$(_dev_run_at "$path" "codex review $base_arg 2>&1"); rc=$?
          else
            out=$(_dev_run_at "$path" "codex review --uncommitted 2>&1"); rc=$?
          fi
          ;;
        opencode)
          out=$(_dev_run_at "$path" "opencode run $(printf '%q' "$review_prompt") 2>&1"); rc=$?
          ;;
        agy)
          out=$(_dev_run_at "$path" "agy -p $(printf '%q' "$review_prompt") 2>&1"); rc=$?
          ;;
        claude)
          out=$(_dev_run_at "$path" "claude --print $(printf '%q' "$review_prompt") 2>&1"); rc=$?
          ;;
        *)
          echo "dev review: unsupported tool '$tool'" >&2; return 1 ;;
      esac
      if [[ -n "$json" ]]; then
        local ok_val="true"; [[ $rc -ne 0 ]] && ok_val="false"
        ${jq} -n -c --arg t "$ref" --arg tool "$tool" --argjson ok "$ok_val" \
          --arg out "$out" '{target:$t,tool:$tool,ok:$ok,lines:($out|split("\n"))}'
      else
        printf '%s\n' "$out"
      fi
      return $rc
    }

    # ============ session management ============
    # List or resume recorded sessions for a project.
    _dev_session() {
      local sub="''${1:-}"; shift || true
      case "$sub" in
        list)
          local ref="''${1:-}" json=""
          [[ "''${1:-}" == --json ]] && { json=1; shift; ref="''${1:-}"; shift || true; } || { ref="''${1:-}"; shift || true; }
          [[ -z "$ref" ]] && { echo "Usage: dev session list [--json] <project>" >&2; return 1; }
          _dev_project_resolve "$ref" || { echo "dev session: unknown project '$ref'" >&2; return 1; }
          local path="$R_PATH"
          # Claude: read per-project sessions-index.json
          local hash; hash=$(printf '%s' "$path" | shasum -a 256 | cut -c1-64 | tr 'a-z' 'A-Z' | sed 's/  .*//' 2>/dev/null)
          # Claude stores the hash as a hex-to-escaped path; use the actual path match
          local idx="" idx_dir
          idx_dir=$(ls -d "$HOME/.claude/projects/"*/ 2>/dev/null | while read -r d; do
            local op; op=$(cat "$d/sessions-index.json" 2>/dev/null | ${jq} -r '.originalPath // ""' 2>/dev/null)
            [[ "$op" == "$path" ]] && { printf '%s' "$d"; break; }
          done)
          [[ -n "$idx_dir" ]] && idx=$(cat "$idx_dir/sessions-index.json" 2>/dev/null)
          if [[ -n "$json" ]]; then
            if [[ -n "$idx" ]]; then
              printf '%s' "$idx" | ${jq} -c '[.entries[]|{sessionId,summary,firstPrompt,gitBranch,created,modified,messageCount}]'
            else
              echo '[]'
            fi
          else
            if [[ -n "$idx" ]]; then
              printf '%s' "$idx" | ${jq} -r '.entries[]|"\(.created[0:16])  \(.messageCount)msg  \(.gitBranch // "-")  \(.summary // .firstPrompt // "-")"' \
                | column -t -s '  ' 2>/dev/null || printf '%s' "$idx" | ${jq} -r '.entries[]|[.created[0:16],.messageCount,.gitBranch // "-",.summary // "-"]|@tsv'
            else
              echo "dev session: no Claude sessions found for '$ref' at $path"
            fi
          fi
          ;;
        resume)
          local ref="''${1:-}" sid="''${2:-}"
          [[ -z "$ref" ]] && { echo "Usage: dev session resume <project> [session_id]" >&2; return 1; }
          _dev_project_resolve "$ref" || { echo "dev session: unknown project '$ref'" >&2; return 1; }
          if [[ -n "$sid" ]]; then
            _dev_agent claude "$ref" --resume "$sid"
          else
            _dev_agent claude "$ref" --resume
          fi
          ;;
        *)
          echo "Usage: dev session <list|resume> <project> [args...]" >&2; return 1 ;;
      esac
    }

    # ============ model catalog ============
    # Show available models per tool.  dev models [tool] [--json]
    _dev_models() {
      local tool="''${1:-}" json=""
      [[ "''${2:-}" == --json ]] && json=1
      case "$tool" in
        claude)
          # Claude models are not dynamically listed via CLI; use well-known list.
          local models='[
            {"name":"claude-sonnet-4-6","alias":"sonnet","tier":"standard"},
            {"name":"claude-opus-4-8","alias":"opus","tier":"powerful"},
            {"name":"claude-haiku-4-5-20251001","alias":"haiku","tier":"fast"},
            {"name":"claude-fable-5","alias":"fable","tier":"ultra"}
          ]'
          if [[ -n "$json" ]]; then printf '%s\n' "$models"
          else printf '%s\n' "$models" | ${jq} -r '.[]|"\(.alias)\t\(.name)\t\(.tier)"' | column -t -s "$(printf '\t')"
          fi
          ;;
        codex)
          local out; out=$(codex debug models 2>/dev/null)
          if [[ -n "$json" ]]; then printf '%s\n' "$out"
          else printf '%s\n' "$out" | ${jq} -r '.[]|"\(.slug)\t\(.name // "")"' 2>/dev/null | column -t -s "$(printf '\t')"
          fi
          ;;
        opencode)
          local out; out=$(opencode models --verbose 2>/dev/null)
          if [[ -n "$json" ]]; then printf '%s\n' "$out"
          else printf '%s\n' "$out" | ${jq} -r '.[]|"\(.id)\t\(.name // "")"' 2>/dev/null | column -t -s "$(printf '\t')"
          fi
          ;;
        "")
          # All tools summary.
          echo "=== claude ===" && _dev_models claude
          echo "=== codex ===" && _dev_models codex
          echo "=== opencode ===" && _dev_models opencode
          ;;
        *)
          echo "dev models: unknown tool '$tool' (claude|codex|opencode)" >&2; return 1 ;;
      esac
    }

    # Poll `dev ps --json`; on an agent entering waiting/error, or a previously
    # seen agent disappearing (finished), push one Telegram notification.
    _dev_usage() {
      local json=0
      while [[ $# -gt 0 ]]; do
        case "$1" in --json) json=1; shift;; *) shift;; esac
      done
      local file="''${XDG_CACHE_HOME:-$HOME/.cache}/claude/usage.json"
      if [[ ! -f "$file" ]]; then
        echo "No usage data yet — start Claude Code and send at least one message." >&2
        exit 1
      fi
      if [[ $json -eq 1 ]]; then
        ${jq} '
          now as $now |
          def normalize_window:
            if . == null then null
            elif (.resets_at != null and .resets_at <= $now) then . + {used_percentage: 0}
            else .
            end;
          . + {
            five_hour: (.five_hour | normalize_window),
            seven_day: (.seven_day | normalize_window)
          }
        ' "$file"
      else
        ${jq} -r '
          def pct: if . == null then "-" else "\(.)%" end;
          now as $now |
          def used($window):
            if $window == null then null
            elif ($window.resets_at != null and $window.resets_at <= $now) then 0
            else $window.used_percentage
            end;
          def jst($window):
            if $window == null or $window.resets_at == null or $window.resets_at <= $now then ""
            else ($window.resets_at + 32400) | strftime(" (resets %m/%d %H:%M JST)")
            end;
          "claude usage",
          "  5h:  \(used(.five_hour) | pct)\(jst(.five_hour))",
          "  7d:  \(used(.seven_day) | pct)\(jst(.seven_day))",
          "  updated: \((.updated_at + 32400) | strftime("%H:%M:%S") // "-") JST"
        ' "$file"
      fi
    }

    _dev_watch() {
      local interval=30 once=""
      while [[ $# -gt 0 ]]; do
        case "$1" in --interval) interval="$2"; shift 2 ;; --once) once=1; shift ;; *) shift ;; esac
      done
      local state="$HOME/.dev/watch-state.json"
      mkdir -p "$HOME/.dev"
      while :; do
        local now prev cur
        now=$(dev ps --json 2>/dev/null)
        prev="{}"; [[ -f "$state" ]] && prev=$(cat "$state" 2>/dev/null)
        cur=$(printf '%s' "$now" | ${jq} -c '[.[]|select(.tool!=null)]|map({key:"\(.target)/\(.tool)",value:.status})|from_entries' 2>/dev/null)
        [[ -z "$cur" ]] && cur="{}"
        # entered waiting/error
        printf '%s' "$cur" | ${jq} -r --argjson prev "$prev" \
          'to_entries[] | select(.value=="waiting" or .value=="error") | select($prev[.key] != .value) | .key + " " + .value' 2>/dev/null \
          | while IFS= read -r line; do [[ -n "$line" ]] && _dev_notify "dev: $line"; done
        # disappeared (finished)
        printf '%s' "$prev" | ${jq} -r --argjson cur "$cur" 'keys[]|select($cur[.]==null)' 2>/dev/null \
          | while IFS= read -r key; do [[ -n "$key" ]] && _dev_notify "dev: $key finished"; done
        printf '%s' "$cur" > "$state"
        [[ -n "$once" ]] && break
        sleep "$interval"
      done
    }

    # ── dev task store helpers ──────────────────────────────────────────────────
    # Storage: ~/.dev/projects/<project-id>/tasks/<task-id>/

    _task_store() { echo "$HOME/.dev/projects"; }
    _task_now_iso() { date -u +%Y-%m-%dT%H:%M:%SZ; }

    # Find the task directory for a given task ID.  Prints the path on success.
    _task_find_tdir() {
      local task_id="''${1:-}" store pdir tdir
      store=$(_task_store)
      for pdir in "$store"/*/; do
        [[ -d "$pdir" ]] || continue
        tdir="''${pdir}tasks/''${task_id}"
        [[ -f "''${tdir}/task.json" ]] && { echo "$tdir"; return 0; }
      done
      return 1
    }

    # Generate the next T-YYYYMMDD-NNN id for a project (daily sequence).
    _task_next_id() {
      local project_id="''${1:-}" dp tdir max d n
      dp=$(date +%Y%m%d)
      tdir="$(_task_store)/''${project_id}/tasks"
      mkdir -p "$tdir"
      max=0
      for d in "$tdir"/T-"$dp"-*/; do
        [[ -d "$d" ]] || continue
        n=$(basename "$d"); n="''${n##T-????????-}"; n=$((10#''${n:-0}))
        [[ $n -gt $max ]] && max=$n
      done
      printf "T-%s-%03d" "$dp" $((max + 1))
    }

    # Generate the next Q-YYYYMMDD-NNN id for a project (daily sequence).
    _question_next_id() {
      local project_id="''${1:-}" dp qfile max line id n
      dp=$(date +%Y%m%d)
      qfile="$(_task_store)/''${project_id}/questions.jsonl"
      max=0
      if [[ -f "$qfile" ]]; then
        while IFS= read -r line; do
          [[ -z "$line" ]] && continue
          id=$(printf '%s' "$line" | ${jq} -r '.id // empty' 2>/dev/null)
          [[ "$id" == Q-"$dp"-* ]] || continue
          n="''${id##Q-????????-}"; n=$((10#''${n:-0})); [[ $n -gt $max ]] && max=$n
        done < "$qfile"
      fi
      printf "Q-%s-%03d" "$dp" $((max + 1))
    }

    # Append an event line to <task_dir>/events.jsonl.
    # Args: task_dir type actor message [extra_json_object]
    _task_event_append() {
      local task_dir="''${1:-}" type="''${2:-}" actor="''${3:-}" message="''${4:-}" extra="''${5:-}" ts ev
      ts=$(_task_now_iso)
      if [[ -n "$extra" ]]; then
        ev=$(printf '%s' "$extra" | ${jq} -c \
          --arg ts "$ts" --arg type "$type" --arg actor "$actor" --arg msg "$message" \
          '. + {ts:$ts,type:$type,actor:$actor,message:$msg}')
      else
        ev=$(${jq} -n -c \
          --arg ts "$ts" --arg type "$type" --arg actor "$actor" --arg msg "$message" \
          '{ts:$ts,type:$type,actor:$actor,message:$msg}')
      fi
      printf '%s\n' "$ev" >> "''${task_dir}/events.jsonl"
    }

    # Set a new phase on task.json and record a phase_changed event.
    _task_phase_set() {
      local task_dir="''${1:-}" new_phase="''${2:-}" actor="''${3:-}" message="''${4:-}" old_phase ts tmp
      old_phase=$(${jq} -r '.phase // "draft"' "''${task_dir}/task.json" 2>/dev/null)
      ts=$(_task_now_iso)
      tmp=$(${jq} --arg phase "$new_phase" --arg ts "$ts" \
        '.phase = $phase | .updated_at = $ts' "''${task_dir}/task.json")
      printf '%s\n' "$tmp" > "''${task_dir}/task.json"
      _task_event_append "$task_dir" "phase_changed" "$actor" "$message" \
        "$(${jq} -n -c --arg from "$old_phase" --arg to "$new_phase" '{from:$from,to:$to}')"
    }

    # Standard JSON success / error envelopes.
    _task_json_ok() {
      ${jq} -n -c --argjson ok true \
        --arg task_id "''${1:-}" --arg project_id "''${2:-}" --arg phase "''${3:-}" --arg message "''${4:-}" \
        '{ok:$ok,task_id:$task_id,project_id:$project_id,phase:$phase,message:$message}'
    }
    _task_json_err() {
      ${jq} -n -c --argjson ok false \
        --arg error "''${1:-}" --arg message "''${2:-}" --arg task_id "''${3:-}" \
        '{ok:$ok,error:$error,message:$message,task_id:$task_id}'
    }

    # Count open blocking questions for a task.
    _task_blocking_questions_open() {
      local project_dir="''${1:-}" task_id="''${2:-}" qfile
      qfile="''${project_dir}/questions.jsonl"
      [[ -f "$qfile" ]] || { echo 0; return; }
      ${jq} -s --arg tid "$task_id" \
        '[.[] | select(.task_id == $tid and .status == "open" and .severity == "blocking")] | length' \
        "$qfile" 2>/dev/null || echo 0
    }

    # Find the project directory that owns a given question ID.  Prints the path.
    _question_find_pdir() {
      local qid="''${1:-}" store pdir qfile lid
      store=$(_task_store)
      for pdir in "$store"/*/; do
        [[ -d "$pdir" ]] || continue
        qfile="''${pdir}questions.jsonl"
        [[ -f "$qfile" ]] || continue
        while IFS= read -r line; do
          [[ -z "$line" ]] && continue
          lid=$(printf '%s' "$line" | ${jq} -r '.id // empty' 2>/dev/null)
          [[ "$lid" == "$qid" ]] && { echo "''${pdir%/}"; return 0; }
        done < "$qfile"
      done
      return 1
    }
  '';

  coderBin = "/opt/homebrew/bin/coder";

  # Wrapper that sources local.zsh before launching opencode so that
  # LLM_CF_TOKEN (and other secrets) are available even when opencode is
  # invoked by a subagent that did not inherit an interactive-shell env.
  opencodeWrapper = pkgs.writeShellScriptBin "opencode" ''
    export PATH="$HOME/.nix-profile/bin:/opt/homebrew/bin:$PATH"
    [[ -f "${localZsh}" ]] && source "${localZsh}"
    exec /opt/homebrew/bin/opencode "$@"
  '';

  coderProxy = pkgs.writeShellScriptBin "coder-proxy" ''
    ${loadEnv}
    CF_TOKEN=$(${pkgs.cloudflared}/bin/cloudflared access token -app "$CODER_CF_APP" 2>/dev/null)
    SESSION=$(cat "${coderSessionPath}" 2>/dev/null)
    exec env \
      CODER_URL="$CODER_URL" \
      CODER_SESSION_TOKEN="$SESSION" \
      CODER_HEADER="CF-Access-Token=$CF_TOKEN" \
      ${coderBin} ssh --stdio --ssh-host-prefix coder. "$@"
  '';

  coderCli = pkgs.writeShellScriptBin "coder-cli" ''
    ${loadEnv}
    CF_TOKEN=$(${pkgs.cloudflared}/bin/cloudflared access token -app "$CODER_CF_APP" 2>/dev/null)
    SESSION=$(cat "${coderSessionPath}" 2>/dev/null)
    exec env \
      CODER_URL="$CODER_URL" \
      CODER_SESSION_TOKEN="$SESSION" \
      CODER_HEADER="CF-Access-Token=$CF_TOKEN" \
      ${coderBin} "$@"
  '';

  # `dev tui` — live fleet TUI (ratatui). A pure client of `dev … --json`:
  # it polls `dev ps --json` and shells out to dev for actions, holding no
  # local/remote/Coder logic of its own. Crate lives in pkgs/dev-tui.
  devTui = pkgs.rustPlatform.buildRustPackage {
    pname = "dev-tui";
    version = "0.1.0";
    src = ../../pkgs/dev-tui;
    cargoLock.lockFile = ../../pkgs/dev-tui/Cargo.lock;
  };

  devCmd = pkgs.writeShellScriptBin "dev" ''
    ${loadConfig}
    ${devProjectFns}

    group="''${1:-}"
    shift || true

    # Hierarchical command dispatch
    case "$group" in

      # Core target management (flat env+project)
      ls)
        _dev_take_flags "$@"; set -- "''${DEV_ARGS[@]}"
        if [[ -n "$JSON" ]]; then _dev_ls_json; exit 0; fi
        echo "ENVS"
        printf "  %-20s %-32s %-20s %-8s %s\n" NAME HOST PROXY SHELL OS
        for entry in "''${DEV_ENVS[@]:-}"; do
          IFS='|' read -r n host proxy shell os <<< "$entry"
          printf "  %-20s %-32s %-20s %-8s %s\n" "$n" "$host" "''${proxy:--}" "''${shell:-bash}" "$(_env_get_os "$n")"
        done
        echo ""
        echo "LOCAL PROJECTS"
        printf "  %-24s %s\n" NAME PATH
        for entry in "''${DEV_LOCAL[@]:-}"; do
          IFS='|' read -r n path <<< "$entry"
          printf "  %-24s %s\n" "$n" "$path"
        done
        echo ""
        echo "REMOTE PROJECTS"
        printf "  %-24s %-12s %-8s %s\n" NAME ENV OS PATH
        for entry in "''${DEV_REMOTE[@]:-}"; do
          IFS='|' read -r n env rp <<< "$entry"
          printf "  %-24s %-12s %-8s %s\n" "$n" "$env" "$(_env_get_os "$env" "$rp")" "$rp"
        done
        ;;

      targets)
        _dev_take_flags "$@"; set -- "''${DEV_ARGS[@]}"
        if [[ -n "$JSON" ]]; then
          _dev_targets_json
        else
          _dev_targets_json | ${jq} -r '.[] | "\(.kind)\t\(.name)\t\(.path // .host // "")"' \
            | while IFS=$'\t' read -r k n p; do printf "%-16s %-26s %s\n" "$k" "$n" "$p"; done
        fi
        ;;

      tools)
        # Backend registry: agent tools known to dev.
        # dev tools [--json]
        _dev_take_flags "$@"
        if [[ -n "$JSON" ]]; then
          _dev_tools_json
        else
          _dev_tools_json | ${jq} -r \
            '.[] | "\(.name)\tdispatch=\(.dispatchable)\treview=\(.review)\tps=\(.ps_detect)\tattach=\(.attach)"' \
            | column -t -s "$(printf '\t')"
        fi
        ;;

      run)
        # Run a command in any env or project (local/remote transparent). Primary
        # agent interface. [--json] must lead (the rest is the command verbatim).
        #   dev run <name> <cmd...>          single target, streams stdout (unchanged)
        #   dev run --all <cmd...>           every project, concurrently
        #   dev run a,b,c <cmd...>           comma list, concurrently
        #   dev run [--json] …               structured [{target,ok,exit,stdout,stderr}]
        runjson=""
        [[ "''${1:-}" == "--json" ]] && { runjson=1; shift; }
        name="''${1:-}"; shift || true
        if [[ -z "$name" ]]; then
          echo "Usage: dev run [--json] <env|project|--all|a,b,c|-> <cmd...>" >&2
          exit 1
        fi
        _targets=(); multi=""
        case "$name" in
          --all) multi=1; while IFS= read -r _t; do _targets+=("$_t"); done < <(_dev_list_projects) ;;
          *,*)   multi=1; IFS=, read -r -a _targets <<< "$name" ;;
          -)     name=$(_dev_select_any); [[ -z "$name" ]] && exit 1; _targets=("$name") ;;
          *)     _targets=("$name") ;;
        esac
        # Join the remaining args into one shell command string ("$*"), so both
        # `dev run p "git status"` and `dev run p git status` work and shell
        # metacharacters (pipes, $(), redirections) are interpreted on the target.
        qcmd="$*"
        if [[ -n "$multi" || -n "$runjson" ]]; then
          _dev_run_fanout "$runjson" "$qcmd" "''${_targets[@]}"
        else
          _dev_resolve_and_run "''${_targets[0]}" "$qcmd"
        fi
        ;;

      shell)
        # Open an interactive shell. Env name → root of env. Project name → project dir.
        name=$(_dev_require_name any "''${1:-}") || exit 1
        _dev_resolve_and_run "$name" "" interactive
        ;;

      code)
        _dev_code "''${1:-}"
        ;;

      info)
        _dev_take_flags "$@"; set -- "''${DEV_ARGS[@]}"
        if [[ -n "$JSON" ]]; then
          name="''${1:-}"
          [[ -z "$name" ]] && name=$(_dev_select_any) || true
          [[ -z "$name" ]] && exit 1
          _dev_info_json "$name"
        else
          _dev_info "''${1:-}"
        fi
        ;;

      _preview)
        # Internal: fzf preview renderer (see _dev_fzf). Not shown in usage.
        _dev_preview "''${1:-}"
        ;;

      doctor)
        _dev_doctor "$@"
        ;;

      # Agent management
      agent)
        agent_cmd="''${1:-}"
        shift || true
        case "$agent_cmd" in
          start)
            tool="''${1:-}"
            shift || true
            if [[ -z "$tool" ]]; then
              echo "Usage: dev agent start <tool> <project> [flags...]" >&2
              echo "  tools: claude, codex, opencode, agy" >&2
              exit 1
            fi
            case "$tool" in
              claude|codex|opencode|agy)
                if [[ -z "''${1:-}" || "''${1:-}" == "-" ]]; then
                  selected=$(_dev_select_project) || exit 1
                  shift || true
                  set -- "$selected" "$@"
                fi
                _dev_agent "$tool" "$@"
                ;;
              *)
                echo "dev agent start: unknown tool '$tool'" >&2
                echo "  Available: claude, codex, opencode, agy" >&2
                exit 1
                ;;
            esac
            ;;
          dispatch)
            _dev_dispatch "$@"
            ;;
          attach)
            _dev_attach "''${1:-}"
            ;;
          logs)
            _dev_logs "$@"
            ;;
          kill)
            _dev_kill "$@"
            ;;
          ps)
            _dev_take_flags "$@"; set -- "''${DEV_ARGS[@]}"
            tmpdir=$(mktemp -d)
            trap 'rm -rf "$tmpdir"' EXIT
            local_order=()
            remote_order=()

            # Local projects — macOS: use lsof to get process cwd
            for entry in "''${DEV_LOCAL[@]:-}"; do
              IFS='|' read -r n lp <<< "$entry"
              local_order+=("$n")
              (
                # claude: Agent View JSON is authoritative — it lists *all* claude
                # sessions (interactive and --bg) with status/kind, and works where
                # `pgrep -x claude` does not (claude runs under node on macOS).
                if command -v claude >/dev/null 2>&1; then
                  cjson=$(claude agents --json --cwd "$lp" 2>/dev/null)
                fi
                [[ -z "''${cjson:-}" ]] && cjson='[]'
                # Save full claude JSON for JSON renderer (session_id/name enrichment).
                printf '%s' "$cjson" > "$tmpdir/cj_L_$n"
                printf '%s' "$cjson" \
                  | ${jq} -r '.[]? | "claude \(.pid) \((.status // "running"))/\((.kind // ""))"' 2>/dev/null \
                  >> "$tmpdir/L_$n"
                cl_pids=" $(printf '%s' "$cjson" | ${jq} -r '.[]?.pid' 2>/dev/null | tr '\n' ' ') "
                # pgrep net: catch any claude the JSON missed, plus the other tools
                # (no JSON interface); match process cwd via lsof, skip dup claude pids.
                for _tool in claude codex opencode agy; do
                  pids=$(pgrep -x "$_tool" 2>/dev/null) || continue
                  while IFS= read -r pid; do
                    [[ "$_tool" == claude && "$cl_pids" == *" $pid "* ]] && continue
                    cwd=$(lsof -p "$pid" -a -d cwd -Fn 2>/dev/null | awk '/^n/{print substr($0,2)}')
                    [[ "$cwd" == "$lp"* ]] && printf '%s %s %s\n' "$_tool" "$pid" "$cwd" >> "$tmpdir/L_$n"
                  done <<< "$pids"
                done
                [[ -s "$tmpdir/L_$n" ]] || echo "stopped" > "$tmpdir/L_$n"
              ) &
            done

            # Remote projects
            for entry in "''${DEV_REMOTE[@]:-}"; do
              IFS='|' read -r n env rp <<< "$entry"
              remote_order+=("$n")
              (
                case "$(_env_get_os "$env" "$rp")" in
                  windows)
                  cjson=$(_dev_windows_claude_agents_json "$env" "$rp" 2>/dev/null)
                  rc=$?
                  if [[ $rc -ne 0 ]]; then
                    echo "unreachable" > "$tmpdir/R_$n"
                  else
                    [[ -z "''${cjson:-}" ]] && cjson='[]'
                    printf '%s' "$cjson" > "$tmpdir/cj_R_$n"
                    printf '%s' "$cjson" \
                      | ${jq} -r '.[]? | "claude \(.pid) \((.status // "running"))/\((.kind // ""))"' 2>/dev/null \
                      >> "$tmpdir/R_$n"
                    others=$(_dev_windows_process_rows "$env" "$rp" 2>/dev/null)
                    [[ -n "$others" ]] && printf '%s\n' "$others" >> "$tmpdir/R_$n"
                    [[ -s "$tmpdir/R_$n" ]] || echo "stopped" > "$tmpdir/R_$n"
                  fi
                  ;;
                  *)
                  # claude: Agent View JSON, run remotely, parsed locally with jq.
                  # Works on macOS remotes (no /proc) and carries status/kind. The
                  # remote stays exit-0 when reachable so rc tracks SSH reachability.
                  cjson=$(_dev_exec_on_env "$env" "" "if command -v claude >/dev/null 2>&1; then claude agents --json --cwd '$rp' 2>/dev/null || echo '[]'; else echo '[]'; fi" 2>/dev/null)
                  rc=$?
                  if [[ $rc -ne 0 ]]; then
                    echo "unreachable" > "$tmpdir/R_$n"
                  else
                    printf '%s' "$cjson" > "$tmpdir/cj_R_$n"
                    printf '%s' "$cjson" \
                      | ${jq} -r '.[]? | "claude \(.pid) \((.status // "running"))/\((.kind // ""))"' 2>/dev/null \
                      >> "$tmpdir/R_$n"
                    # other tools: no JSON interface (native binaries → pgrep works);
                    # resolve cwd via /proc on Linux, lsof elsewhere (macOS remotes).
                    others=$(_dev_exec_on_env "$env" "" "
                      for _tool in codex opencode agy; do
                        pids=\$(pgrep -x \"\$_tool\" 2>/dev/null) || continue
                        while IFS= read -r pid; do
                          cwd=\$(readlink /proc/\$pid/cwd 2>/dev/null || lsof -p \"\$pid\" -a -d cwd -Fn 2>/dev/null | awk '/^n/{print substr(\$0,2)}')
                          [[ \"\$cwd\" == \"$rp\"* ]] && printf '%s %s %s\n' \"\$_tool\" \"\$pid\" \"\$cwd\"
                        done <<< \"\$pids\"
                      done
                    " 2>/dev/null)
                    [[ -n "$others" ]] && printf '%s\n' "$others" >> "$tmpdir/R_$n"
                    [[ -s "$tmpdir/R_$n" ]] || echo "stopped" > "$tmpdir/R_$n"
                  fi
                  ;;
                esac
              ) &
            done
            wait

            _ps_print_row() {
              local key="$1" n="$2"
              result=$(cat "$tmpdir/$key" 2>/dev/null || echo "?")
              case "$result" in
                stopped|unreachable|"?"|"n/a"*)
                  printf "%-24s %s\n" "$n" "$result"
                  ;;
                *)
                  first=true
                  while IFS= read -r line; do
                    [[ -z "$line" ]] && continue
                    _tool=''${line%% *}; rest=''${line#* }; pid=''${rest%% *}; cwd=''${rest#* }
                    if $first; then
                      printf "%-24s %-10s pid=%-8s %s\n" "$n" "$_tool" "$pid" "$cwd"; first=false
                    else
                      printf "%-24s %-10s pid=%-8s %s\n" "" "$_tool" "$pid" "$cwd"
                    fi
                  done <<< "$result"
                  ;;
              esac
            }

            # JSON renderer: re-parse the per-target tmpdir text into one object per
            # agent. Shares the collection above; the human path (_ps_print_row) is
            # untouched. claude rows carry status/kind (text drops cwd → null);
            # pgrep rows carry cwd (status="running"); sentinels carry only status.
            _dev_ps_rows_json() {
              local key="$1" n="$2" loc="$3" result line tool rest pid detail st kd
              result=$(cat "$tmpdir/$key" 2>/dev/null || echo "?")
              case "$result" in
                stopped|unreachable|"?"|"n/a"*)
                  ${jq} -n -c --arg t "$n" --arg loc "$loc" --arg st "$result" \
                    '{target:$t,location:$loc,tool:null,pid:null,status:$st,kind:null,cwd:null,session_id:null,name:null}' ;;
                *)
                  while IFS= read -r line; do
                    [[ -z "$line" ]] && continue
                    tool=''${line%% *}; rest=''${line#* }; pid=''${rest%% *}; detail=''${rest#* }
                    if [[ "$tool" == claude ]]; then
                      st=''${detail%%/*}; kd=''${detail#*/}
                      [[ -z "$st" || "$st" == "null" ]] && st=running
                      [[ "$kd" == "null" ]] && kd=""
                      local cjf="$tmpdir/cj_$key" sid="" nm=""
                      if [[ -f "$cjf" ]]; then
                        sid=$(${jq} -r --argjson p "$pid" '.[]?|select(.pid==$p)|.sessionId // ""' "$cjf" 2>/dev/null)
                        nm=$(${jq} -r --argjson p "$pid" '.[]?|select(.pid==$p)|.name // ""' "$cjf" 2>/dev/null)
                      fi
                      ${jq} -n -c --arg t "$n" --arg loc "$loc" --argjson pid "$pid" --arg st "$st" --arg kd "$kd" \
                        --arg sid "$sid" --arg nm "$nm" \
                        '{target:$t,location:$loc,tool:"claude",pid:$pid,status:$st,kind:$kd,cwd:null,session_id:$sid,name:$nm}'
                    else
                      ${jq} -n -c --arg t "$n" --arg loc "$loc" --arg tool "$tool" --argjson pid "$pid" --arg cwd "$detail" \
                        '{target:$t,location:$loc,tool:$tool,pid:$pid,status:"running",kind:null,cwd:$cwd,session_id:null,name:null}'
                    fi
                  done <<< "$result" ;;
              esac
            }

            if [[ -n "$JSON" ]]; then
              {
                for n in "''${local_order[@]}";  do _dev_ps_rows_json "L_$n" "$n" local;  done
                for n in "''${remote_order[@]}"; do _dev_ps_rows_json "R_$n" "$n" remote; done
              } | ${jq} -s '.'
              exit 0
            fi

            printf "%-24s %-10s %s\n" "LOCAL PROJECT" "TOOL" "PID  STATUS/CWD"
            printf "%-24s %-10s %s\n" "-------------" "----" "---------"
            for n in "''${local_order[@]}"; do _ps_print_row "L_$n" "$n"; done
            echo ""
            printf "%-24s %-10s %s\n" "REMOTE PROJECT" "TOOL" "PID  STATUS/CWD"
            printf "%-24s %-10s %s\n" "--------------" "----" "---------"
            for n in "''${remote_order[@]}"; do _ps_print_row "R_$n" "$n"; done
            ;;
          review)
            _dev_review "$@"
            ;;
          watch)
            _dev_watch "$@"
            ;;
          *)
            echo "Usage: dev agent <command> [args...]" >&2
            echo "" >&2
            echo "  start <tool> <target> [flags]  Start agent (claude/codex/opencode/agy)" >&2
            echo "  dispatch <target> [flags]      Launch background agent" >&2
            echo "  attach <target|id>             Attach to agent" >&2
            echo "  logs <target|id> [-f]          Tail agent log" >&2
            echo "  kill <target|id>               Stop agent" >&2
            echo "  ps [--json]                    List running agents" >&2
            echo "  review <target> [flags]        Code review" >&2
            echo "  watch [--interval N]           Watch for agent state changes" >&2
            exit 1
            ;;
        esac
        ;;

      # Session management
      session)
        _dev_session "$@"
        ;;

      # Git operations
      git)
        git_cmd="''${1:-}"
        shift || true
        case "$git_cmd" in
          status)
            _dev_take_flags "$@"; set -- "''${DEV_ARGS[@]}"
            if [[ $# -gt 0 ]]; then
              names=()
              for arg in "$@"; do
                if [[ "$arg" == "-" ]]; then
                  while IFS= read -r n; do names+=("$n"); done < <(_dev_select_many_projects)
                else
                  names+=("$arg")
                fi
              done
            else
              names=()
              while IFS= read -r n; do names+=("$n"); done < <(_dev_list_projects)
            fi
            if [[ -n "$JSON" ]]; then _dev_status_json "''${names[@]}"; exit 0; fi
            for name in "''${names[@]}"; do
              if lp=$(_local_get_path "$name" 2>/dev/null); then
                { echo "=== LOCAL: $name ==="; git -C "$lp" log --oneline -2 2>/dev/null; git -C "$lp" status --short 2>/dev/null; } &
              elif env_name=$(_remote_get_env "$name" 2>/dev/null); then
                rp=$(_remote_get_path "$name")
                { echo "=== REMOTE: $name ==="; _dev_remote_git_human "$env_name" "$rp" 2>/dev/null; } &
              fi
            done
            wait
            ;;
          diff)
            _dev_diff "$@"
            ;;
          worktree)
            worktree_cmd="''${1:-}"
            shift || true
            case "$worktree_cmd" in
              ls|rm)
                _dev_worktree "$worktree_cmd" "$@"
                ;;
              *)
                echo "Usage: dev git worktree <ls|rm> <project> [branch]" >&2
                exit 1
                ;;
            esac
            ;;
          pr)
            _dev_pr "$@"
            ;;
          *)
            echo "Usage: dev git <command> [args...]" >&2
            echo "" >&2
            echo "  status [target...] [--json]    Git status" >&2
            echo "  diff <target> [--stat] [--json]  Show diff" >&2
            echo "  worktree ls <target>           List worktrees" >&2
            echo "  worktree rm <target> <branch>  Remove worktree" >&2
            echo "  pr <target> [flags]            Create pull request" >&2
            exit 1
            ;;
        esac
        ;;

      # TUI/Dashboard
      tui)
        exec ${devTui}/bin/dev-tui "$@"
        ;;

      dash)
        # Interim fleet dashboard over `dev agent ps --json`. Live refresh with ctrl-r;
        # row actions (enter/ctrl-l/ctrl-k/ctrl-d) shell out to dev — they light
        # up once L2 (attach/logs/kill/dispatch) lands. Field {1} is the target.
        command -v fzf >/dev/null 2>&1 || { echo "dev dash: fzf is required" >&2; exit 1; }
        dev _dashrows | fzf --reverse --height=100% \
          --header 'enter:attach  ctrl-l:logs  ctrl-k:kill  ctrl-r:refresh' \
          --preview 'dev _preview {1}' --preview-window=right,50%,wrap \
          --bind 'ctrl-r:reload(dev _dashrows)' \
          --bind 'enter:become(dev agent attach {1})' \
          --bind 'ctrl-l:execute(dev agent logs {1} -f)' \
          --bind 'ctrl-k:execute(dev agent kill {1})+reload(dev _dashrows)'
        ;;

      # Utilities
      tools)
        # Backend registry: agent tools known to dev.
        # dev tools [--json]
        _dev_take_flags "$@"
        if [[ -n "$JSON" ]]; then
          _dev_tools_json
        else
          _dev_tools_json | ${jq} -r \
            '.[] | "\(.name)\tdispatch=\(.dispatchable)\treview=\(.review)\tps=\(.ps_detect)\tattach=\(.attach)"' \
            | column -t -s "$(printf '\t')"
        fi
        ;;

      models)
        _dev_models "''${1:-}" "''${2:-}"
        ;;

      run)
        # Run a command in any env or project (local/remote transparent). Primary
        # agent interface. [--json] must lead (the rest is the command verbatim).
        #   dev run <name> <cmd...>          single target, streams stdout (unchanged)
        #   dev run --all <cmd...>           every project, concurrently
        #   dev run a,b,c <cmd...>           comma list, concurrently
        #   dev run [--json] …               structured [{target,ok,exit,stdout,stderr}]
        runjson=""
        [[ "''${1:-}" == "--json" ]] && { runjson=1; shift; }
        name="''${1:-}"; shift || true
        if [[ -z "$name" ]]; then
          echo "Usage: dev run [--json] <env|project|--all|a,b,c|-> <cmd...>" >&2
          exit 1
        fi
        _targets=(); multi=""
        case "$name" in
          --all) multi=1; while IFS= read -r _t; do _targets+=("$_t"); done < <(_dev_list_projects) ;;
          *,*)   multi=1; IFS=, read -r -a _targets <<< "$name" ;;
          -)     name=$(_dev_select_any); [[ -z "$name" ]] && exit 1; _targets=("$name") ;;
          *)     _targets=("$name") ;;
        esac
        # Join the remaining args into one shell command string ("$*"), so both
        # `dev run p "git status"` and `dev run p git status` work and shell
        # metacharacters (pipes, $(), redirections) are interpreted on the target.
        qcmd="$*"
        if [[ -n "$multi" || -n "$runjson" ]]; then
          _dev_run_fanout "$runjson" "$qcmd" "''${_targets[@]}"
        else
          _dev_resolve_and_run "''${_targets[0]}" "$qcmd"
        fi
        ;;

      notify)
        [[ -z "''${1:-}" ]] && { echo "Usage: dev notify <message...>" >&2; exit 1; }
        _dev_notify "$*"
        ;;

      usage)
        _dev_usage "$@"
        ;;

      # Task orchestration (Phase 1: store + CLI)
      task)
        task_sub="''${1:-}"
        shift || true
        case "$task_sub" in

          # dev task new <project> --title <title> [--brief <text>] [--json]
          new)
            project_id="''${1:-}"; shift || true
            title=""; brief_text=""; JSON=""
            while [[ $# -gt 0 ]]; do
              case "''${1:-}" in
                --title) title="''${2:-}"; shift 2 ;;
                --brief) brief_text="''${2:-}"; shift 2 ;;
                --json)  JSON=1; shift ;;
                *) shift ;;
              esac
            done
            if [[ -z "$project_id" || -z "$title" ]]; then
              echo "Usage: dev task new <project> --title <title> [--brief <text>] [--json]" >&2; exit 1
            fi
            pdir="$(_task_store)/''${project_id}"
            task_id=$(_task_next_id "$project_id")
            tdir="''${pdir}/tasks/''${task_id}"
            mkdir -p "''${tdir}/reviews" "''${tdir}/test-results"
            ts=$(_task_now_iso)
            ${jq} -n \
              --arg id "$task_id" --arg pid "$project_id" --arg title "$title" --arg ts "$ts" \
              '{id:$id,project_id:$pid,title:$title,phase:"draft",priority:"normal",
                created_at:$ts,updated_at:$ts,created_by:"human",
                assigned_tool:null,assigned_model:null,
                worktree_branch:null,worktree_path:null,
                scope:{paths:[],allowed_paths:[],forbidden_paths:[],risk:"unknown"},
                validation:{commands:[],required:true},
                links:{run_id:null,session_id:null,pr_url:null},
                summary:{latest_question:null,latest_handoff:null,diff_files:[],
                         review_status:"none",test_status:"unknown"}}' \
              > "''${tdir}/task.json"
            [[ -n "$brief_text" ]] && printf '%s\n' "$brief_text" > "''${tdir}/brief.md"
            if [[ ! -f "''${pdir}/project.json" ]]; then
              ${jq} -n --arg id "$project_id" --arg ts "$ts" \
                '{id:$id,target:$id,location:"local",env:null,path:null,created_at:$ts,updated_at:$ts}' \
                > "''${pdir}/project.json"
            fi
            _task_event_append "$tdir" "task_created" "human" "task created"
            if [[ -n "$JSON" ]]; then _task_json_ok "$task_id" "$project_id" "draft" "task created"
            else echo "$task_id"; fi
            ;;

          # dev task list [<project>] [--phase <phase>] [--json]
          list)
            filter_project=""; filter_phase=""; JSON=""
            while [[ $# -gt 0 ]]; do
              case "''${1:-}" in
                --phase) filter_phase="''${2:-}"; shift 2 ;;
                --json)  JSON=1; shift ;;
                -*)      shift ;;
                *)       filter_project="''${1:-}"; shift ;;
              esac
            done
            store=$(_task_store)
            tmpf=$(mktemp)
            for pdir in "$store"/*/; do
              [[ -d "$pdir" ]] || continue
              pname=$(basename "$pdir")
              [[ -n "$filter_project" && "$pname" != "$filter_project" ]] && continue
              [[ -d "''${pdir}tasks" ]] || continue
              for tdir in "''${pdir}tasks"/T-*/; do
                [[ -f "''${tdir}task.json" ]] || continue
                if [[ -n "$filter_phase" ]]; then
                  tp=$(${jq} -r '.phase // ""' "''${tdir}task.json" 2>/dev/null)
                  [[ "$tp" == "$filter_phase" ]] || continue
                fi
                cat "''${tdir}task.json" >> "$tmpf"
                printf '\n' >> "$tmpf"
              done
            done
            if [[ -n "$JSON" ]]; then
              ${jq} -s '.' "$tmpf"
            else
              while IFS= read -r line; do
                [[ -z "$line" ]] && continue
                printf "%-18s %-12s %-22s %s\n" \
                  "$(printf '%s' "$line" | ${jq} -r '.id')" \
                  "$(printf '%s' "$line" | ${jq} -r '.phase')" \
                  "$(printf '%s' "$line" | ${jq} -r '.project_id')" \
                  "$(printf '%s' "$line" | ${jq} -r '.title')"
              done < <(${jq} -c '.[]' "$tmpf" 2>/dev/null)
            fi
            rm -f "$tmpf"
            ;;

          # dev task show <task-id> [--json]
          show)
            _dev_take_flags "$@"; set -- "''${DEV_ARGS[@]}"
            task_id="''${1:-}"
            if [[ -z "$task_id" ]]; then echo "Usage: dev task show <task-id> [--json]" >&2; exit 1; fi
            tdir=$(_task_find_tdir "$task_id") || {
              [[ -n "$JSON" ]] && _task_json_err "not_found" "task not found" "$task_id" || echo "not found: $task_id" >&2; exit 1
            }
            if [[ -n "$JSON" ]]; then
              cat "''${tdir}/task.json"
            else
              ${jq} -r '"id:         " + .id,
                        "title:      " + .title,
                        "phase:      " + .phase,
                        "project:    " + .project_id,
                        "priority:   " + .priority,
                        "created_at: " + .created_at,
                        "tool:       " + (.assigned_tool // "-"),
                        "model:      " + (.assigned_model // "-")' "''${tdir}/task.json"
              [[ -f "''${tdir}/brief.md" ]] && { echo ""; echo "brief:"; cat "''${tdir}/brief.md"; }
              [[ -f "''${tdir}/approved-plan.md" ]] && { echo ""; echo "plan: approved-plan.md (approved)"; }
              [[ ! -f "''${tdir}/approved-plan.md" && -f "''${tdir}/plan.md" ]] && { echo ""; echo "plan: plan.md (draft)"; }
            fi
            ;;

          # dev task context <task-id> [--json|--markdown]
          context)
            task_id="''${1:-}"; ctx_mode="markdown"
            shift || true
            case "''${1:-}" in --json) ctx_mode=json ;; --markdown) ctx_mode=markdown ;; esac
            if [[ -z "$task_id" ]]; then echo "Usage: dev task context <task-id> [--json|--markdown]" >&2; exit 1; fi
            tdir=$(_task_find_tdir "$task_id") || { echo "not found: $task_id" >&2; exit 1; }
            pname=$(${jq} -r '.project_id' "''${tdir}/task.json")
            pdir="$(_task_store)/''${pname}"
            if [[ "$ctx_mode" == "json" ]]; then
              ${jq} -n \
                --argjson task "$(cat "''${tdir}/task.json")" \
                --arg project_md "$(cat "''${pdir}/project.md" 2>/dev/null || true)" \
                --arg project_plan "$(cat "''${pdir}/plan.md" 2>/dev/null || true)" \
                --arg brief "$(cat "''${tdir}/brief.md" 2>/dev/null || true)" \
                --arg plan "$(cat "''${tdir}/approved-plan.md" 2>/dev/null || cat "''${tdir}/plan.md" 2>/dev/null || true)" \
                --arg decisions "$(cat "''${pdir}/decisions.jsonl" 2>/dev/null || true)" \
                '{task:$task,project_md:$project_md,project_plan:$project_plan,brief:$brief,plan:$plan,decisions:$decisions}'
            else
              ${jq} -r '"# Task Context: " + .id + "\n\n**Task:** " + .title + "  \n**Phase:** " + .phase + "  \n**Project:** " + .project_id' "''${tdir}/task.json"
              echo ""
              [[ -f "''${pdir}/project.md" ]] && { echo "## Project"; cat "''${pdir}/project.md"; echo ""; }
              [[ -f "''${pdir}/plan.md" ]] && { echo "## Project Plan"; cat "''${pdir}/plan.md"; echo ""; }
              [[ -f "''${tdir}/brief.md" ]] && { echo "## Brief"; cat "''${tdir}/brief.md"; echo ""; }
              if [[ -f "''${tdir}/approved-plan.md" ]]; then
                echo "## Approved Plan"; cat "''${tdir}/approved-plan.md"; echo ""
              elif [[ -f "''${tdir}/plan.md" ]]; then
                echo "## Plan (draft)"; cat "''${tdir}/plan.md"; echo ""
              fi
              qfile="''${pdir}/questions.jsonl"
              if [[ -f "$qfile" ]]; then
                oqs=$(${jq} -s --arg tid "$task_id" \
                  '[.[] | select(.task_id == $tid and .status == "open")]' "$qfile" 2>/dev/null || echo '[]')
                if [[ "$oqs" != "[]" ]]; then
                  echo "## Open Questions"
                  printf '%s' "$oqs" | ${jq} -r '.[] | "- [\(.severity)] \(.id): \(.question)"'
                  echo ""
                fi
              fi
              echo "## Scope"
              ${jq} -r '.scope | "risk: " + .risk' "''${tdir}/task.json"
              allowed=$(${jq} -r '.scope.allowed_paths | join(", ")' "''${tdir}/task.json")
              forbidden=$(${jq} -r '.scope.forbidden_paths | join(", ")' "''${tdir}/task.json")
              [[ -n "$allowed" ]] && echo "allowed: $allowed"
              [[ -n "$forbidden" ]] && echo "forbidden: $forbidden"
              echo ""
              echo "## Validation"
              ${jq} -r '.validation.commands[] // empty' "''${tdir}/task.json" 2>/dev/null
            fi
            ;;

          # dev task events <task-id> [--json]
          events)
            _dev_take_flags "$@"; set -- "''${DEV_ARGS[@]}"
            task_id="''${1:-}"
            if [[ -z "$task_id" ]]; then echo "Usage: dev task events <task-id> [--json]" >&2; exit 1; fi
            tdir=$(_task_find_tdir "$task_id") || { echo "not found: $task_id" >&2; exit 1; }
            evfile="''${tdir}/events.jsonl"
            if [[ ! -f "$evfile" ]]; then
              [[ -n "$JSON" ]] && echo "[]" || echo "(no events)"; exit 0
            fi
            if [[ -n "$JSON" ]]; then ${jq} -s '.' "$evfile"
            else
              while IFS= read -r line; do
                [[ -z "$line" ]] && continue
                printf '%s' "$line" | ${jq} -r '"[\(.ts)] \(.type) [\(.actor)] \(.message)"' 2>/dev/null
              done < "$evfile"
            fi
            ;;

          # dev task ask <task-id> <question> [--category <c>] [--severity <s>] [--json]
          ask)
            task_id="''${1:-}"; shift || true
            question="''${1:-}"; shift || true
            category="behavior"; severity="blocking"; options_json="[]"; recommendation=""; context_text=""; JSON=""
            while [[ $# -gt 0 ]]; do
              case "''${1:-}" in
                --category)       category="''${2:-}"; shift 2 ;;
                --severity)       severity="''${2:-}"; shift 2 ;;
                --options)        options_json="''${2:-}"; shift 2 ;;
                --recommendation) recommendation="''${2:-}"; shift 2 ;;
                --context)        context_text="''${2:-}"; shift 2 ;;
                --json)           JSON=1; shift ;;
                *) shift ;;
              esac
            done
            if [[ -z "$task_id" || -z "$question" ]]; then
              echo "Usage: dev task ask <task-id> <question> [--category <c>] [--severity blocking|nonblocking|note] [--json]" >&2; exit 1
            fi
            tdir=$(_task_find_tdir "$task_id") || {
              [[ -n "$JSON" ]] && _task_json_err "not_found" "task not found" "$task_id" || echo "not found: $task_id" >&2; exit 1
            }
            pname=$(${jq} -r '.project_id' "''${tdir}/task.json")
            pdir="$(_task_store)/''${pname}"
            qid=$(_question_next_id "$pname")
            ts=$(_task_now_iso)
            ${jq} -n -c \
              --arg id "$qid" --arg task_id "$task_id" --arg pid "$pname" \
              --arg question "$question" --arg category "$category" --arg severity "$severity" \
              --argjson options "$options_json" \
              --arg rec "$recommendation" --arg ctx "$context_text" --arg ts "$ts" \
              '{id:$id,task_id:$task_id,project_id:$pid,status:"open",severity:$severity,
                category:$category,question:$question,options:$options,
                agent_recommendation:($rec|if .=="" then null else . end),
                context:$ctx,created_at:$ts,answered_at:null,answer:null}' \
              >> "''${pdir}/questions.jsonl"
            _task_event_append "$tdir" "question_opened" "agent" "question $qid: $question"
            [[ "$severity" == "blocking" ]] && _task_phase_set "$tdir" "needs_spec" "dev" "blocking question opened"
            phase=$(${jq} -r '.phase' "''${tdir}/task.json")
            if [[ -n "$JSON" ]]; then
              ${jq} -n -c --argjson ok true \
                --arg qid "$qid" --arg tid "$task_id" --arg pid "$pname" --arg phase "$phase" \
                '{ok:$ok,question_id:$qid,task_id:$tid,project_id:$pid,phase:$phase,message:("question " + $qid + " opened")}'
            else
              echo "$qid"
            fi
            ;;

          # dev task answer <question-id> <answer> [--json]
          answer)
            qid="''${1:-}"; shift || true
            answer_text="''${1:-}"; shift || true
            JSON=""; [[ "''${1:-}" == "--json" ]] && { JSON=1; shift; }
            if [[ -z "$qid" || -z "$answer_text" ]]; then
              echo "Usage: dev task answer <question-id> <answer> [--json]" >&2; exit 1
            fi
            pdir=$(_question_find_pdir "$qid") || {
              [[ -n "$JSON" ]] && ${jq} -n -c '{ok:false,error:"not_found",message:"question not found"}' || echo "not found: $qid" >&2; exit 1
            }
            ts=$(_task_now_iso)
            qfile="''${pdir}/questions.jsonl"
            tmpf=$(mktemp)
            found_task_id=""
            while IFS= read -r line; do
              [[ -z "$line" ]] && continue
              lid=$(printf '%s' "$line" | ${jq} -r '.id // empty' 2>/dev/null)
              if [[ "$lid" == "$qid" ]]; then
                found_task_id=$(printf '%s' "$line" | ${jq} -r '.task_id // empty')
                printf '%s' "$line" | ${jq} -c \
                  --arg ans "$answer_text" --arg ts "$ts" \
                  '.status = "answered" | .answer = $ans | .answered_at = $ts'
              else
                printf '%s\n' "$line"
              fi
            done < "$qfile" > "$tmpf" && mv "$tmpf" "$qfile"
            if [[ -n "$found_task_id" ]]; then
              ans_tdir=$(_task_find_tdir "$found_task_id") || true
              if [[ -n "$ans_tdir" ]]; then
                _task_event_append "$ans_tdir" "question_answered" "human" "question $qid answered"
                ans_phase=$(${jq} -r '.phase' "''${ans_tdir}/task.json")
                if [[ "$ans_phase" == "needs_spec" ]]; then
                  remaining=$(_task_blocking_questions_open "$pdir" "$found_task_id")
                  [[ "$remaining" == "0" ]] && _task_phase_set "$ans_tdir" "planning" "dev" "blocking questions resolved"
                fi
              fi
            fi
            if [[ -n "$JSON" ]]; then
              ${jq} -n -c --argjson ok true --arg qid "$qid" --arg ans "$answer_text" \
                '{ok:$ok,question_id:$qid,message:"question answered",answer:$ans}'
            else
              echo "answered: $qid"
            fi
            ;;

          # dev task write-plan <task-id> [--file <path>] [--json]
          write-plan)
            task_id="''${1:-}"; shift || true
            file_path=""; JSON=""
            while [[ $# -gt 0 ]]; do
              case "''${1:-}" in
                --file) file_path="''${2:-}"; shift 2 ;;
                --json) JSON=1; shift ;;
                *) shift ;;
              esac
            done
            if [[ -z "$task_id" ]]; then echo "Usage: dev task write-plan <task-id> [--file <path>] [--json]" >&2; exit 1; fi
            tdir=$(_task_find_tdir "$task_id") || {
              [[ -n "$JSON" ]] && _task_json_err "not_found" "task not found" "$task_id" || echo "not found: $task_id" >&2; exit 1
            }
            pname=$(${jq} -r '.project_id' "''${tdir}/task.json")
            pdir="$(_task_store)/''${pname}"
            if [[ -n "$file_path" ]]; then
              plan_content=$(cat "$file_path") || { echo "cannot read: $file_path" >&2; exit 1; }
            else
              plan_content=$(cat)
            fi
            printf '%s\n' "$plan_content" > "''${tdir}/plan.md"
            _task_event_append "$tdir" "plan_written" "agent" "plan written"
            blocking=$(_task_blocking_questions_open "$pdir" "$task_id")
            new_phase=$([[ "$blocking" -gt 0 ]] && echo "needs_spec" || echo "planned")
            _task_phase_set "$tdir" "$new_phase" "dev" "plan written"
            [[ -n "$JSON" ]] && _task_json_ok "$task_id" "$pname" "$new_phase" "plan written" || echo "plan written → $new_phase"
            ;;

          # dev task approve <task-id> [--json]
          approve)
            _dev_take_flags "$@"; set -- "''${DEV_ARGS[@]}"
            task_id="''${1:-}"
            if [[ -z "$task_id" ]]; then echo "Usage: dev task approve <task-id> [--json]" >&2; exit 1; fi
            tdir=$(_task_find_tdir "$task_id") || {
              [[ -n "$JSON" ]] && _task_json_err "not_found" "task not found" "$task_id" || echo "not found: $task_id" >&2; exit 1
            }
            pname=$(${jq} -r '.project_id' "''${tdir}/task.json")
            pdir="$(_task_store)/''${pname}"
            if [[ ! -f "''${tdir}/plan.md" ]]; then
              [[ -n "$JSON" ]] && _task_json_err "plan_missing" "plan.md not found — run write-plan first" "$task_id" || echo "error: plan.md missing" >&2; exit 1
            fi
            blocking=$(_task_blocking_questions_open "$pdir" "$task_id")
            if [[ "$blocking" -gt 0 ]]; then
              [[ -n "$JSON" ]] && _task_json_err "blocking_questions_open" "cannot approve while blocking questions are open" "$task_id" || echo "error: $blocking blocking question(s) open" >&2; exit 1
            fi
            cp "''${tdir}/plan.md" "''${tdir}/approved-plan.md"
            _task_event_append "$tdir" "plan_approved" "human" "plan approved"
            _task_phase_set "$tdir" "approved" "human" "plan approved"
            [[ -n "$JSON" ]] && _task_json_ok "$task_id" "$pname" "approved" "plan approved" || echo "approved: $task_id"
            ;;

          # dev task reject <task-id> [--reason <text>] [--json]
          reject)
            task_id="''${1:-}"; shift || true
            reason=""; JSON=""
            while [[ $# -gt 0 ]]; do
              case "''${1:-}" in
                --reason) reason="''${2:-}"; shift 2 ;;
                --json)   JSON=1; shift ;;
                *) shift ;;
              esac
            done
            if [[ -z "$task_id" ]]; then echo "Usage: dev task reject <task-id> [--reason <text>] [--json]" >&2; exit 1; fi
            tdir=$(_task_find_tdir "$task_id") || {
              [[ -n "$JSON" ]] && _task_json_err "not_found" "task not found" "$task_id" || echo "not found: $task_id" >&2; exit 1
            }
            pname=$(${jq} -r '.project_id' "''${tdir}/task.json")
            reject_msg="task rejected''${reason:+: $reason}"
            _task_event_append "$tdir" "task_rejected" "human" "$reject_msg"
            _task_phase_set "$tdir" "rejected" "human" "$reject_msg"
            [[ -n "$JSON" ]] && _task_json_ok "$task_id" "$pname" "rejected" "$reject_msg" || echo "rejected: $task_id"
            ;;

          # dev task plan <task-id> [--tool <tool>] [--model <model>] [--json]
          plan)
            task_id="''${1:-}"; shift || true
            plan_tool="claude"; plan_model=""; JSON=""
            while [[ $# -gt 0 ]]; do
              case "''${1:-}" in
                --tool)  plan_tool="''${2:-}";  shift 2 ;;
                --model) plan_model="''${2:-}"; shift 2 ;;
                --json)  JSON=1; shift ;;
                *) shift ;;
              esac
            done
            if [[ -z "$task_id" ]]; then echo "Usage: dev task plan <task-id> [--tool <t>] [--model <m>] [--json]" >&2; exit 1; fi
            tdir=$(_task_find_tdir "$task_id") || {
              [[ -n "$JSON" ]] && _task_json_err "not_found" "task not found" "$task_id" || echo "not found: $task_id" >&2; exit 1
            }
            pname=$(${jq} -r '.project_id' "''${tdir}/task.json")
            _task_phase_set "$tdir" "planning" "dev" "planning agent dispatched"
            _task_event_append "$tdir" "agent_dispatched" "dev" "planning agent dispatched (tool=$plan_tool)"
            plan_prompt="You are planning dev task $task_id for project $pname.

Rules:
- Do not edit files.
- Read the shared task context: dev task context $task_id --markdown
- If behavior, scope, compatibility, API, UX, migration, release, or validation is ambiguous, run:
    dev task ask $task_id \"<question>\" --category <category> --severity blocking
  and stop.
- If there are no blocking questions, write the plan:
    dev task write-plan $task_id
- The plan must include: understanding, proposed behavior, files to touch, files not to touch, implementation steps, validation, risks, rollback.
- Do not implement until the task is approved."
            if [[ -n "$plan_model" ]]; then
              _dev_dispatch "$pname" --tool "$plan_tool" --model "$plan_model" "$plan_prompt"
            else
              _dev_dispatch "$pname" --tool "$plan_tool" "$plan_prompt"
            fi
            if [[ -n "$JSON" ]]; then
              _task_json_ok "$task_id" "$pname" "planning" "planning agent dispatched"
            else
              echo "planning: $task_id (project=$pname tool=$plan_tool)"
            fi
            ;;

          *)
            echo "Usage: dev task <command> [args...]" >&2
            echo "" >&2
            echo "  new <project> --title <title> [--brief <text>]   Create task" >&2
            echo "  list [project] [--phase <phase>] [--json]         List tasks" >&2
            echo "  show <task-id> [--json]                           Show task" >&2
            echo "  context <task-id> [--markdown|--json]             Agent context" >&2
            echo "  events <task-id> [--json]                         Task events" >&2
            echo "  ask <task-id> <question> [--category <c>] [--severity <s>]" >&2
            echo "  answer <question-id> <answer> [--json]            Answer question" >&2
            echo "  write-plan <task-id> [--file <path>] [--json]     Save plan (agent)" >&2
            echo "  approve <task-id> [--json]                        Approve plan" >&2
            echo "  reject <task-id> [--reason <text>] [--json]       Reject task" >&2
            echo "  plan <task-id> [--tool <t>] [--model <m>]         Start planning agent" >&2
            exit 1
            ;;
        esac
        ;;

      # Internal commands
      _dashrows)
        # Internal: aligned one-line-per-agent rows for `dev dash` (and reload).
        dev agent ps --json \
          | ${jq} -r '.[] | [.target,(.tool // "-"),.status,((.pid // "-")|tostring),(.cwd // "")] | @tsv' \
          | column -t -s "$(printf '\t')"
        ;;

      *)
        echo "Usage: dev <command> [args...]" >&2
        echo "" >&2
        echo "Core:" >&2
        echo "  ls [--json]                   List envs and projects" >&2
        echo "  info [target] [--json]        Show target details" >&2
        echo "  shell <target>                Interactive shell" >&2
        echo "  code <target>                 Open in VS Code" >&2
        echo "  run [--json] <target> <cmd>   Run command" >&2
        echo "  doctor [--connect]            Validate configuration" >&2
        echo "" >&2
        echo "Agent:" >&2
        echo "  agent start <tool> <target>   Start agent (claude/codex/opencode/agy)" >&2
        echo "  agent dispatch <target> ...   Launch background agent" >&2
        echo "  agent attach <target|id>      Attach to agent" >&2
        echo "  agent logs <target|id> [-f]   Tail agent log" >&2
        echo "  agent kill <target|id>        Stop agent" >&2
        echo "  agent ps [--json]             List running agents" >&2
        echo "  agent review <target> ...     Code review" >&2
        echo "  agent watch [--interval N]    Watch agent state" >&2
        echo "" >&2
        echo "Session:" >&2
        echo "  session <list|resume> ...     Manage sessions" >&2
        echo "" >&2
        echo "Git:" >&2
        echo "  git status [target...]        Git status" >&2
        echo "  git diff <target> [--stat]    Show diff" >&2
        echo "  git worktree <ls|rm> ...      Manage worktrees" >&2
        echo "  git pr <target> ...           Create pull request" >&2
        echo "" >&2
        echo "Task:" >&2
        echo "  task new <project> --title <t> Create task" >&2
        echo "  task list [project] [--phase]  List tasks" >&2
        echo "  task show <task-id>            Show task details" >&2
        echo "  task context <task-id>         Agent context" >&2
        echo "  task ask <task-id> <question>  Open question" >&2
        echo "  task answer <q-id> <answer>    Answer question" >&2
        echo "  task write-plan <task-id>      Save plan (agent)" >&2
        echo "  task approve <task-id>         Approve plan" >&2
        echo "  task reject <task-id>          Reject task" >&2
        echo "" >&2
        echo "TUI:" >&2
        echo "  tui                           Live fleet TUI" >&2
        echo "  dash                          Live fleet dashboard (fzf)" >&2
        echo "" >&2
        echo "Utilities:" >&2
        echo "  tools [--json]                List available tools" >&2
        echo "  models [tool] [--json]        List models per tool" >&2
        echo "  usage [--json]                Show usage statistics" >&2
        echo "  notify <message>              Send notification" >&2
        exit 1
        ;;
    esac
  '';

in
{
  home.packages = [
    coderProxy
    coderCli
    devCmd
    devTui
    opencodeWrapper
  ];

  # ~/.claude/statusline.sh — piped JSON from Claude Code on each API response.
  # Caches rate_limits to ~/.cache/claude/usage.json for `dev usage` and the TUI.
  home.file.".claude/statusline.sh" = {
    executable = true;
    text = ''
      #!/usr/bin/env bash
      input=$(cat)
      cache_dir="''${XDG_CACHE_HOME:-$HOME/.cache}/claude"
      cache_file="$cache_dir/usage.json"
      mkdir -p "$cache_dir"
      printf '%s' "$input" | ${jq} '
        now as $now |
        def normalize_window:
          if . == null then null
          elif (.resets_at != null and .resets_at <= $now) then . + {used_percentage: 0}
          else .
          end;
        {
          updated_at: $now,
          five_hour: (.rate_limits.five_hour | normalize_window),
          seven_day: (.rate_limits.seven_day | normalize_window)
        }
      ' > "$cache_file.tmp" && mv "$cache_file.tmp" "$cache_file"
      five_hour=$(${jq} -r '.five_hour.used_percentage // empty' "$cache_file")
      seven_day=$(${jq} -r '.seven_day.used_percentage // empty' "$cache_file")
      if [[ -n "$five_hour" ]]; then
        printf '5h:%.0f%% 7d:%.0f%%\n' "$five_hour" "$seven_day"
      fi
    '';
  };

  # Inject statusLine into ~/.claude/settings.json (merged, not replaced).
  # Claude Code writes this file itself so we cannot manage it as a symlink —
  # activation merge is the right approach: idempotent, survives Claude Code's writes.
  home.activation.claudeStatusline = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    settings="$HOME/.claude/settings.json"
    script="$HOME/.claude/statusline.sh"
    if [[ -f "$settings" ]]; then
      run ${jq} --arg cmd "$script" \
        '. + {statusLine: {type: "command", command: $cmd}}' \
        "$settings" > "$settings.tmp" && mv "$settings.tmp" "$settings"
    else
      run ${jq} -n --arg cmd "$script" \
        '{statusLine: {type: "command", command: $cmd}}' \
        > "$settings"
    fi
  '';

  programs.ssh.settings = {
    "coder.*" = {
      ConnectTimeout = 0;
      StrictHostKeyChecking = "accept-new";
      UserKnownHostsFile = "~/.ssh/known_hosts.coder";
      LogLevel = "ERROR";
      ProxyCommand = "${coderProxy}/bin/coder-proxy %h";
    };
    "*.coder" = {
      ConnectTimeout = 0;
      StrictHostKeyChecking = "accept-new";
      UserKnownHostsFile = "~/.ssh/known_hosts.coder";
      LogLevel = "ERROR";
      ProxyCommand = "${coderProxy}/bin/coder-proxy %h";
    };
  };
}
