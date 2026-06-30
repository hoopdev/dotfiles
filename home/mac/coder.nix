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
  # DEV_SSH_AGENT — env names whose SSH forwards the local agent onward (so `ssh`
  #              from the remote reuses local keys). The agent itself is whatever
  #              $SSH_AUTH_SOCK points at (1Password); DEV_SSH_AGENT_SOCK overrides
  #              the socket loadConfig falls back to when SSH_AUTH_SOCK is unset.
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
    # when unset. Inside a forwarded SSH session sshd already set it to the
    # caller's agent — leave that untouched.
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
        -o ControlPersist=60s
      )
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
      local raw="''${1:-}" name lp env_name rp ssh_host proxy shell
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
        echo "TYPE   remote project"
        echo "ENV    $env_name"
        echo "HOST   $ssh_host"
        echo "SHELL  $shell"
        echo "PATH   $rp"
        echo ""
        echo "(git status: dev info $name)"
      elif _env_exists "$name"; then
        ssh_host=$(_env_get_host "$name")
        proxy=$(_env_get_proxy "$name")
        shell=$(_env_get_shell "$name")
        echo "TYPE   env"
        echo "HOST   $ssh_host"
        echo "PROXY  ''${proxy:--}"
        echo "SHELL  $shell"
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
        local entry n host proxy shell path env rp _host _shell
        for entry in "''${DEV_LOCAL[@]:-}"; do
          IFS='|' read -r n path <<< "$entry"; [[ -z "$n" ]] && continue
          ${jq} -n --arg name "$n" --arg path "$path" \
            '{name:$name,kind:"local-project",env:null,host:null,shell:null,proxy:null,path:$path}'
        done
        for entry in "''${DEV_REMOTE[@]:-}"; do
          IFS='|' read -r n env rp <<< "$entry"; [[ -z "$n" ]] && continue
          _host=$(_env_get_host "$env" 2>/dev/null); _shell=$(_env_get_shell "$env" 2>/dev/null)
          ${jq} -n --arg name "$n" --arg env "$env" --arg host "$_host" --arg shell "$_shell" --arg path "$rp" \
            '{name:$name,kind:"remote-project",env:$env,host:$host,shell:$shell,proxy:null,path:$path}'
        done
        for entry in "''${DEV_ENVS[@]:-}"; do
          IFS='|' read -r n host proxy shell <<< "$entry"; [[ -z "$n" ]] && continue
          ${jq} -n --arg name "$n" --arg host "$host" --arg proxy "$proxy" --arg shell "''${shell:-bash}" \
            '{name:$name,kind:"env",env:$name,host:$host,shell:$shell,proxy:$proxy,path:null}'
        done
      } | ${jq} -s '.'
    }

    # dev ls --json — grouped to mirror the human sections.
    _dev_ls_json() {
      _dev_targets_json | ${jq} '{
        envs:   [.[] | select(.kind=="env")            | {name,host,proxy,shell}],
        local:  [.[] | select(.kind=="local-project")  | {name,path}],
        remote: [.[] | select(.kind=="remote-project") | {name,env,path}]
      }'
    }

    # dev info --json — single object (re-resolves; does not touch the human path).
    _dev_info_json() {
      local name="$1" lp env_name rp ssh_host proxy shell
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
          '{name:$name,kind:"local-project",env:null,host:null,proxy:null,shell:null,path:$path,
            git:(if $gitok then {branch:$branch,head:$head,changes:($changes|tonumber)} else null end)}'
      elif env_name=$(_remote_get_env "$name" 2>/dev/null); then
        rp=$(_remote_get_path "$name"); ssh_host=$(_env_get_host "$env_name")
        proxy=$(_env_get_proxy "$env_name"); shell=$(_env_get_shell "$env_name")
        if [[ "$shell" != pwsh && "$shell" != nu ]]; then
          rg=$(_dev_exec_on_env "$env_name" "$rp" "echo \"B:\$(git branch --show-current 2>/dev/null)\"; echo \"H:\$(git log --oneline -1 2>/dev/null)\"; echo \"C:\$(git status --short 2>/dev/null | wc -l | tr -d ' ')\"" 2>/dev/null) && {
            gitok=true
            while IFS= read -r _l; do
              case "$_l" in B:*) branch="''${_l#B:}" ;; H:*) head="''${_l#H:}" ;; C:*) changes="''${_l#C:}" ;; esac
            done <<< "$rg"
          }
        fi
        ${jq} -n --arg name "$name" --arg env "$env_name" --arg host "$ssh_host" --arg proxy "$proxy" \
          --arg shell "$shell" --arg path "$rp" --argjson gitok "$gitok" \
          --arg branch "$branch" --arg head "$head" --arg changes "''${changes:-0}" \
          '{name:$name,kind:"remote-project",env:$env,host:$host,proxy:$proxy,shell:$shell,path:$path,
            git:(if $gitok then {branch:$branch,head:$head,changes:($changes|tonumber)} else null end)}'
      elif _env_exists "$name"; then
        ssh_host=$(_env_get_host "$name"); proxy=$(_env_get_proxy "$name"); shell=$(_env_get_shell "$name")
        ${jq} -n --arg name "$name" --arg host "$ssh_host" --arg proxy "$proxy" --arg shell "$shell" \
          '{name:$name,kind:"env",env:$name,host:$host,proxy:$proxy,shell:$shell,path:null,git:null}'
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
            if rg=$(_dev_exec_on_env "$env_name" "$rp" "echo \"B:\$(git branch --show-current 2>/dev/null)\"; echo \"H:\$(git log --oneline -1 2>/dev/null)\"; echo \"C:\$(git status --short 2>/dev/null | wc -l | tr -d ' ')\"" 2>/dev/null); then
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
      _dev_run_at "$R_PATH" "ls -t \"\$HOME/.dev/runs/\"*.meta 2>/dev/null | while IFS= read -r p; do grep -q $q \"\$p\" && { cat \"\$p\"; break; }; done"
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
      local tool=claude worktree="" json="" project="" task=""
      while [[ $# -gt 0 ]]; do
        case "$1" in
          --tool) tool="$2"; shift 2 ;;
          --worktree) worktree="$2"; shift 2 ;;
          --json) json=1; shift ;;
          *) if [[ -z "$project" ]]; then project="$1"; else task="''${task:+$task }$1"; fi; shift ;;
        esac
      done
      [[ -z "$project" || -z "$task" ]] && { echo "Usage: dev dispatch <project> [--tool claude|codex|opencode] [--worktree <branch>] \"<task>\"" >&2; return 1; }
      _dev_project_resolve "$project" || { echo "dev dispatch: unknown project '$project'" >&2; return 1; }
      local cwd="$R_PATH" branch="" wt="" id="$tool-$project-$(date +%s)"
      if [[ -n "$worktree" ]]; then branch="$worktree"; wt=$(_dev_worktree_ensure "$R_PATH" "$worktree"); cwd="$wt"; fi
      local qtask qcwd pid="" session=""
      qtask=$(printf '%q' "$task"); qcwd=$(printf '%q' "$cwd")
      case "$tool" in
        claude)
          _dev_run_at "$cwd" "mkdir -p \"\$HOME/.dev/runs\"; claude --bg -p $qtask >/dev/null 2>&1 </dev/null || true"
          local cj
          cj=$(_dev_run_at "$cwd" "claude agents --json --cwd $qcwd 2>/dev/null")
          session=$(printf '%s' "$cj" | ${jq} -r 'sort_by(.startedAt)|last|.sessionId // ""' 2>/dev/null)
          pid=$(printf '%s' "$cj" | ${jq} -r 'sort_by(.startedAt)|last|.pid // ""' 2>/dev/null)
          ;;
        codex|opencode)
          # nohup (not setsid — absent on macOS) detaches from SIGHUP so the
          # agent survives the SSH session / shell exiting; logged to the registry.
          local sub; [[ "$tool" == codex ]] && sub="exec" || sub="run"
          _dev_run_at "$cwd" "mkdir -p \"\$HOME/.dev/runs\"; nohup $tool $sub $qtask >\"\$HOME/.dev/runs/$id.log\" 2>&1 </dev/null & disown 2>/dev/null; true"
          ;;
        *) echo "dev dispatch: unknown tool '$tool'" >&2; return 1 ;;
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
      case "$tool" in
        claude)  if [[ -n "$session" ]]; then _dev_agent claude "$ref" --resume "$session"; else _dev_agent claude "$ref"; fi ;;
        codex)   _dev_agent codex "$ref" resume --last ;;
        opencode) _dev_agent opencode "$ref" --continue ;;
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

    # Poll `dev ps --json`; on an agent entering waiting/error, or a previously
    # seen agent disappearing (finished), push one Telegram notification.
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

  # `dev top` — live fleet TUI (ratatui). A pure client of `dev … --json`:
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

    subcmd="''${1:-}"
    shift || true

    case "$subcmd" in

      ls)
        _dev_take_flags "$@"; set -- "''${DEV_ARGS[@]}"
        if [[ -n "$JSON" ]]; then _dev_ls_json; exit 0; fi
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

      targets)
        _dev_take_flags "$@"; set -- "''${DEV_ARGS[@]}"
        if [[ -n "$JSON" ]]; then
          _dev_targets_json
        else
          _dev_targets_json | ${jq} -r '.[] | "\(.kind)\t\(.name)\t\(.path // .host // "")"' \
            | while IFS=$'\t' read -r k n p; do printf "%-16s %-26s %s\n" "$k" "$n" "$p"; done
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

      notify)
        [[ -z "''${1:-}" ]] && { echo "Usage: dev notify <message...>" >&2; exit 1; }
        _dev_notify "$*"
        ;;

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
            { echo "=== REMOTE: $name ==="; _dev_exec_on_env "$env_name" "$rp" "git log --oneline -2 && git status --short" 2>/dev/null; } &
          fi
        done
        wait
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
            printf '%s' "$cjson" \
              | ${jq} -r '.[]? | "claude \(.pid) \(.status)/\(.kind)"' 2>/dev/null \
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
            case "$(_env_get_shell "$env")" in
              pwsh|nu)
              echo "n/a (non-POSIX remote)" > "$tmpdir/R_$n" ;;
              *)
              # claude: Agent View JSON, run remotely, parsed locally with jq.
              # Works on macOS remotes (no /proc) and carries status/kind. The
              # remote stays exit-0 when reachable so rc tracks SSH reachability.
              cjson=$(_dev_exec_on_env "$env" "" "if command -v claude >/dev/null 2>&1; then claude agents --json --cwd '$rp' 2>/dev/null || echo '[]'; else echo '[]'; fi" 2>/dev/null)
              rc=$?
              if [[ $rc -ne 0 ]]; then
                echo "unreachable" > "$tmpdir/R_$n"
              else
                printf '%s' "$cjson" \
                  | ${jq} -r '.[]? | "claude \(.pid) \(.status)/\(.kind)"' 2>/dev/null \
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
                '{target:$t,location:$loc,tool:null,pid:null,status:$st,kind:null,cwd:null}' ;;
            *)
              while IFS= read -r line; do
                [[ -z "$line" ]] && continue
                tool=''${line%% *}; rest=''${line#* }; pid=''${rest%% *}; detail=''${rest#* }
                if [[ "$tool" == claude ]]; then
                  st=''${detail%%/*}; kd=''${detail#*/}
                  ${jq} -n -c --arg t "$n" --arg loc "$loc" --argjson pid "$pid" --arg st "$st" --arg kd "$kd" \
                    '{target:$t,location:$loc,tool:"claude",pid:$pid,status:$st,kind:$kd,cwd:null}'
                else
                  ${jq} -n -c --arg t "$n" --arg loc "$loc" --arg tool "$tool" --argjson pid "$pid" --arg cwd "$detail" \
                    '{target:$t,location:$loc,tool:$tool,pid:$pid,status:"running",kind:null,cwd:$cwd}'
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

      _dashrows)
        # Internal: aligned one-line-per-agent rows for `dev dash` (and reload).
        dev ps --json \
          | ${jq} -r '.[] | [.target,(.tool // "-"),.status,((.pid // "-")|tostring),(.cwd // "")] | @tsv' \
          | column -t -s "$(printf '\t')"
        ;;

      dash)
        # Interim fleet dashboard over `dev ps --json`. Live refresh with ctrl-r;
        # row actions (enter/ctrl-l/ctrl-k/ctrl-d) shell out to dev — they light
        # up once L2 (attach/logs/kill/dispatch) lands. Field {1} is the target.
        command -v fzf >/dev/null 2>&1 || { echo "dev dash: fzf is required" >&2; exit 1; }
        dev _dashrows | fzf --reverse --height=100% \
          --header 'enter:attach  ctrl-l:logs  ctrl-k:kill  ctrl-r:refresh' \
          --preview 'dev _preview {1}' --preview-window=right,50%,wrap \
          --bind 'ctrl-r:reload(dev _dashrows)' \
          --bind 'enter:become(dev attach {1})' \
          --bind 'ctrl-l:execute(dev logs {1} -f)' \
          --bind 'ctrl-k:execute(dev kill {1})+reload(dev _dashrows)'
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

      worktree)
        _dev_worktree "$@"
        ;;

      diff)
        _dev_diff "$@"
        ;;

      pr)
        _dev_pr "$@"
        ;;

      watch)
        _dev_watch "$@"
        ;;

      top)
        exec ${devTui}/bin/dev-tui "$@"
        ;;

      *)
        echo "Usage: dev <subcommand> [args...]" >&2
        echo "" >&2
        echo "  ls [--json]               List envs and projects" >&2
        echo "  targets [--json]          Flat enumeration of every project + env" >&2
        echo "  run [--json] <name|--all|a,b,c> <cmd>  Run command (single streams; multi fans out)" >&2
        echo "  shell <env|project>       Interactive shell (env: root, project: project dir)" >&2
        echo "  code <project>            Open project in VS Code" >&2
        echo "  info [env|project] [--json]  Show resolved target details" >&2
        echo "  doctor [--connect]        Validate tools, config, and optionally connectivity" >&2
        echo "  claude   <project>        Start Claude Code in project dir" >&2
        echo "  codex    <project>        Start OpenAI Codex in project dir" >&2
        echo "  opencode <project>        Start opencode in project dir" >&2
        echo "  agy      <project>        Start antigravity in project dir" >&2
        echo "  dispatch <project> [--tool t] [--worktree b] <task>  Launch background agent" >&2
        echo "  attach <project|id>       Attach to a dispatched agent" >&2
        echo "  logs <project|id> [-f]    Tail a dispatched agent log" >&2
        echo "  kill <project|id>         Stop a dispatched agent" >&2
        echo "  worktree <list|rm> <project> [branch]  Manage agent worktrees" >&2
        echo "  diff <project> [--stat] [--json]  Diff a (worktree-aware) project" >&2
        echo "  pr <project> [--title t] [--base b] [--draft]  Push branch + open PR" >&2
        echo "  watch [--interval N] [--once]  Notify on agent waiting/finished" >&2
        echo "  status [project...] [--json]  Git status" >&2
        echo "  ps [--json]               Agent process status (local + remote projects)" >&2
        echo "  notify <message...>       Send a Telegram push" >&2
        echo "  dash                      Live fleet dashboard (fzf over ps)" >&2
        echo "  top                       Live fleet TUI (ratatui)" >&2
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
