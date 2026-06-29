{ pkgs, config, ... }:
let
  coderSessionPath = "${config.home.homeDirectory}/Library/Application Support/coderv2/session";
  # Runtime config lives in ~/.config/zsh/local.zsh (not tracked in git).
  #
  # DEV_ENVS   — SSH connection targets: "name|user@host|proxy_cmd|shell"
  #              proxy_cmd: empty = direct SSH; %h = hostname placeholder.
  #              shell: bash (default), zsh, pwsh, nu
  # DEV_LOCAL  — Local projects:  "name|path"
  # DEV_REMOTE — Remote projects: "name|env_name|remote_path"
  #              env_name must be a name in DEV_ENVS.
  # DEV_SSH_AGENT — env names that authenticate via the 1Password SSH agent and
  #              forward it onward (so `ssh` from the remote reuses local keys).
  #              DEV_SSH_AGENT_SOCK optionally overrides the agent socket path.
  #
  # Example:
  #   DEV_ENVS=(
  #     "myenv|user@myenv.example.com|coder-proxy %h|bash"
  #     "win-machine|user@win.ts.net||pwsh"
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
      local target="$1" entry n host proxy shell
      for entry in "''${DEV_ENVS[@]:-}"; do
        IFS='|' read -r n host proxy shell <<< "$entry"
        [[ "$n" == "$target" ]] && { echo "$host"; return 0; }
      done; return 1
    }
    _env_get_proxy() {
      local target="$1" entry n host proxy shell
      for entry in "''${DEV_ENVS[@]:-}"; do
        IFS='|' read -r n host proxy shell <<< "$entry"
        [[ "$n" == "$target" ]] && { echo "$proxy"; return 0; }
      done; return 0
    }
    _env_get_shell() {
      local target="$1" entry n host proxy shell
      for entry in "''${DEV_ENVS[@]:-}"; do
        IFS='|' read -r n host proxy shell <<< "$entry"
        [[ "$n" == "$target" ]] && { echo "''${shell:-bash}"; return 0; }
      done; echo "bash"
    }
    _env_exists() {
      local target="$1" entry n host proxy shell
      for entry in "''${DEV_ENVS[@]:-}"; do
        IFS='|' read -r n host proxy shell <<< "$entry"
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
      for entry in "''${DEV_ENVS[@]:-}"; do IFS='|' read -r n x x x <<< "$entry"; echo "$n"; done
    }

    _dev_select_project() {
      if ! command -v fzf >/dev/null 2>&1; then
        echo "dev: fzf is required when project name is omitted" >&2
        return 1
      fi
      _dev_list_projects | fzf --prompt='dev project> ' --height=40% --reverse
    }
    _dev_select_any() {
      if ! command -v fzf >/dev/null 2>&1; then
        echo "dev: fzf is required when name is omitted" >&2
        return 1
      fi
      {
        for n in $(_dev_list_projects); do echo "project $n"; done
        for n in $(_dev_list_envs); do echo "env     $n"; done
      } | fzf --prompt='dev> ' --height=40% --reverse | awk '{print $2}'
    }
    _dev_select_many_projects() {
      if ! command -v fzf >/dev/null 2>&1; then
        echo "dev: fzf is required for '-' selection" >&2
        return 1
      fi
      _dev_list_projects | fzf --multi --prompt='dev projects> ' --height=40% --reverse
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
      local -a ssh_opts=(-o StrictHostKeyChecking=accept-new -o "UserKnownHostsFile=~/.ssh/known_hosts.coder")
      # 1Password SSH agent auth + forwarding for envs listed in DEV_SSH_AGENT
      # (set in local.zsh). IdentityAgent authenticates this hop with 1Password;
      # ForwardAgent forwards the same agent so onward `ssh` from the remote
      # reuses the local 1Password keys (prompts still appear on this Mac).
      # Socket defaults to the macOS 1Password path; override via DEV_SSH_AGENT_SOCK.
      local _agent_e _sock
      for _agent_e in "''${DEV_SSH_AGENT[@]:-}"; do
        if [[ "$_agent_e" == "$env_name" ]]; then
          _sock="''${DEV_SSH_AGENT_SOCK:-$HOME/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock}"
          ssh_opts+=(-o "IdentityAgent=\"$_sock\"" -o "ForwardAgent=\"$_sock\"")
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

    _dev_info() {
      local name="$1" lp env_name rp ssh_host proxy shell
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
        echo "NAME    $name"
        echo "TYPE    remote project"
        echo "ENV     $env_name"
        echo "HOST    $ssh_host"
        echo "PROXY   ''${proxy:--}"
        echo "SHELL   $shell"
        echo "PATH    $rp"
        if [[ "$shell" == "pwsh" || "$shell" == "nu" ]]; then
          echo "GIT     n/a ($shell remote)"
        else
          echo "GIT"
          _dev_exec_on_env "$env_name" "$rp" "
            git log --oneline -1 2>/dev/null | sed 's/^/  head    /'
            git branch --show-current 2>/dev/null | sed 's/^/  branch  /'
            printf '  changes '
            git status --short 2>/dev/null | wc -l | tr -d ' '
            printf '\n'
          " 2>/dev/null || echo "  unreachable"
        fi
      elif _env_exists "$name"; then
        ssh_host=$(_env_get_host "$name")
        proxy=$(_env_get_proxy "$name")
        shell=$(_env_get_shell "$name")
        echo "NAME    $name"
        echo "TYPE    env"
        echo "HOST    $ssh_host"
        echo "PROXY   ''${proxy:--}"
        echo "SHELL   $shell"
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
      local entry n host proxy shell path env rp name lp env_name
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
        IFS='|' read -r n host proxy shell <<< "$entry"
        [[ -n "$n" && -n "$host" ]] && _ok "env $n -> $host" || _fail "bad DEV_ENVS entry: $entry"
        case "''${shell:-bash}" in bash|zsh|pwsh|nu) : ;; *) _warn "env $n has unknown shell '$shell'" ;; esac
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

      if [[ ''${#DEV_SSH_AGENT[@]:-0} -gt 0 ]]; then
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

  devCmd = pkgs.writeShellScriptBin "dev" ''
    ${loadConfig}
    ${devProjectFns}

    subcmd="''${1:-}"
    shift || true

    case "$subcmd" in

      ls)
        echo "ENVS"
        printf "  %-20s %-32s %-20s %s\n" NAME HOST PROXY SHELL
        for entry in "''${DEV_ENVS[@]:-}"; do
          IFS='|' read -r n host proxy shell <<< "$entry"
          printf "  %-20s %-32s %-20s %s\n" "$n" "$host" "''${proxy:--}" "''${shell:-bash}"
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
        printf "  %-24s %-12s %s\n" NAME ENV PATH
        for entry in "''${DEV_REMOTE[@]:-}"; do
          IFS='|' read -r n env rp <<< "$entry"
          printf "  %-24s %-12s %s\n" "$n" "$env" "$rp"
        done
        ;;

      run)
        # Run a command in any env or project (local/remote transparent). Primary agent interface.
        name="$1"; shift || true
        if [[ -z "$name" ]]; then
          echo "Usage: dev run <env|project|-> <cmd...>" >&2
          echo "       dev run - <cmd...>    Select target with fzf" >&2
          exit 1
        fi
        [[ "$name" == "-" ]] && name=$(_dev_select_any)
        [[ -z "$name" ]] && exit 1
        _dev_resolve_and_run "$name" "$(printf '%q ' "$@")"
        ;;

      shell)
        # Open an interactive shell. Env name → root of env. Project name → project dir.
        name=$(_dev_require_name any "''${1:-}") || exit 1
        _dev_resolve_and_run "$name" "" interactive
        ;;

      claude|codex|opencode|agy)
        if [[ -z "''${1:-}" || "''${1:-}" == "-" ]]; then
          selected=$(_dev_select_project) || exit 1
          shift || true
          set -- "$selected" "$@"
        fi
        _dev_agent "$subcmd" "$@"
        ;;

      code)
        _dev_code "''${1:-}"
        ;;

      info)
        _dev_info "''${1:-}"
        ;;

      doctor)
        _dev_doctor "$@"
        ;;

      status)
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
        for name in "''${names[@]}"; do
          if lp=$(_local_get_path "$name" 2>/dev/null); then
            { echo "=== LOCAL: $name ==="; git -C "$lp" log --oneline -2 2>/dev/null; git -C "$lp" status --short 2>/dev/null; } &
          elif env_name=$(_remote_get_env "$name" 2>/dev/null); then
            rp=$(_remote_get_path "$name")
            { echo "=== REMOTE: $name ==="; _dev_exec_on_env "$env_name" "$rp" "git log --oneline -2 && git status --short" 2>/dev/null; } &
          fi
        done
        wait
        ;;

      ps)
        tmpdir=$(mktemp -d)
        trap 'rm -rf "$tmpdir"' EXIT
        local_order=()
        remote_order=()

        # Local projects — macOS: use lsof to get process cwd
        for entry in "''${DEV_LOCAL[@]:-}"; do
          IFS='|' read -r n lp <<< "$entry"
          local_order+=("$n")
          (
            for _tool in claude codex opencode agy; do
              pids=$(pgrep -x "$_tool" 2>/dev/null) || continue
              while IFS= read -r pid; do
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
            case "$(_env_get_shell "$env")" in
              pwsh|nu)
              echo "n/a (non-POSIX remote)" > "$tmpdir/R_$n" ;;
              *)
              info=$(_dev_exec_on_env "$env" "" "
                for _tool in claude codex opencode agy; do
                  pids=\$(pgrep -x \"\$_tool\" 2>/dev/null) || continue
                  while IFS= read -r pid; do
                    cwd=\$(readlink /proc/\$pid/cwd 2>/dev/null || echo '?')
                    [[ \"\$cwd\" == \"$rp\"* ]] && printf '%s %s %s\n' \"\$_tool\" \"\$pid\" \"\$cwd\"
                  done <<< \"\$pids\"
                done
              " 2>/dev/null)
              rc=$?
              if [[ $rc -ne 0 ]]; then
                echo "unreachable" > "$tmpdir/R_$n"
              elif [[ -n "$info" ]]; then
                echo "$info" > "$tmpdir/R_$n"
              else
                echo "stopped" > "$tmpdir/R_$n"
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
            stopped|unreachable|"?"|"n/a (Windows)")
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

        printf "%-24s %-10s %s\n" "LOCAL PROJECT" "TOOL" "PID / CWD"
        printf "%-24s %-10s %s\n" "-------------" "----" "---------"
        for n in "''${local_order[@]}"; do _ps_print_row "L_$n" "$n"; done
        echo ""
        printf "%-24s %-10s %s\n" "REMOTE PROJECT" "TOOL" "PID / CWD"
        printf "%-24s %-10s %s\n" "--------------" "----" "---------"
        for n in "''${remote_order[@]}"; do _ps_print_row "R_$n" "$n"; done
        ;;

      *)
        echo "Usage: dev <subcommand> [args...]" >&2
        echo "" >&2
        echo "  ls                        List envs and projects" >&2
        echo "  run  <env|project> <cmd>  Run command (env: no cd, project: cd to dir)" >&2
        echo "  shell <env|project>       Interactive shell (env: root, project: project dir)" >&2
        echo "  code <project>            Open project in VS Code" >&2
        echo "  info [env|project]        Show resolved target details" >&2
        echo "  doctor [--connect]        Validate tools, config, and optionally connectivity" >&2
        echo "  claude   <project>        Start Claude Code in project dir" >&2
        echo "  codex    <project>        Start OpenAI Codex in project dir" >&2
        echo "  opencode <project>        Start opencode in project dir" >&2
        echo "  agy      <project>        Start antigravity in project dir" >&2
        echo "  status [project...]       Git status" >&2
        echo "  ps                        Claude process status (local + remote projects)" >&2
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
    opencodeWrapper
  ];

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
