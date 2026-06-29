{ pkgs, config, ... }:
let
  coderSessionPath = "${config.home.homeDirectory}/Library/Application Support/coderv2/session";
  # Runtime config lives in ~/.config/zsh/local.zsh (not tracked in git).
  #
  # DEV_ENVS   — SSH connection targets: "name|user@host|proxy_cmd|shell"
  #              proxy_cmd: empty = direct SSH; %h = hostname placeholder.
  #              shell: bash (default), zsh, pwsh
  # DEV_LOCAL  — Local projects:  "name|path"
  # DEV_REMOTE — Remote projects: "name|env_name|remote_path"
  #              env_name must be a name in DEV_ENVS.
  #
  # Example:
  #   DEV_ENVS=(
  #     "myenv|user@myenv.example.com|coder-proxy %h|bash"
  #     "win-machine|user@win.ts.net||pwsh"
  #   )
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
      [[ -n "$proxy" ]] && ssh_opts+=(-o "ProxyCommand=$proxy")
      if [[ "$shell" == "pwsh" ]]; then
        local ps_cmd=""
        [[ -n "$rp" ]] && ps_cmd="Set-Location '$rp'; "
        if [[ -n "$interactive" ]]; then
          ssh "''${ssh_opts[@]}" "$flag" "$ssh_host" "''${ps_cmd}pwsh -NoLogo"
        else
          ssh "''${ssh_opts[@]}" "$flag" "$ssh_host" "pwsh -NoLogo -NonInteractive -Command \"''${ps_cmd}''${cmd}\""
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
        name="$1"; shift
        [[ -z "$name" ]] && { echo "Usage: dev run <env|project> <cmd...>" >&2; exit 1; }
        _dev_resolve_and_run "$name" "$(printf '%q ' "$@")"
        ;;

      shell)
        # Open an interactive shell. Env name → root of env. Project name → project dir.
        name="$1"
        [[ -z "$name" ]] && { echo "Usage: dev shell <env|project>" >&2; exit 1; }
        _dev_resolve_and_run "$name" "" interactive
        ;;

      claude|codex|opencode|agy)
        _dev_agent "$subcmd" "$@"
        ;;

      status)
        if [[ $# -gt 0 ]]; then
          names=("$@")
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
            if [[ "$(_env_get_shell "$env")" == "pwsh" ]]; then
              echo "n/a (Windows)" > "$tmpdir/R_$n"
            else
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
            fi
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
