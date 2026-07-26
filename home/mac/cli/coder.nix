{
  lib,
  pkgs,
  ...
}:
let
  # local.zsh (untracked) may override these values. The 1Password fallback keeps
  # the deployment URL and Cloudflare application URL out of the repository.
  localZsh = "$HOME/.config/zsh/local.zsh";
  coderCfAppCache = "$HOME/.cache/coder/cf-app";
  onePasswordItem = "op://Personal/Coder";

  # Read the relatively stable application name from 1Password once, then use
  # a mode-600 local cache. Short-lived Cloudflare access tokens are still
  # generated on demand by `cloudflared`; only the app identifier is cached.
  loadCoderCfApp = ''
    if [[ -z "''${CODER_CF_APP:-}" ]]; then
      if [[ -r "${coderCfAppCache}" ]]; then
        export CODER_CF_APP="$(<"${coderCfAppCache}")"
      elif command -v op >/dev/null 2>&1; then
        app="$(op read "${onePasswordItem}/CODER_CF_APP" 2>/dev/null || true)"
        if [[ -n "$app" ]]; then
          mkdir -p "$(dirname "${coderCfAppCache}")"
          print -r -- "$app" > "${coderCfAppCache}"
          chmod 600 "${coderCfAppCache}"
          export CODER_CF_APP="$app"
        fi
      fi
    fi
  '';

  coderCfHeader = pkgs.writeTextFile {
    name = "coder-cf-header";
    destination = "/bin/coder-cf-header";
    executable = true;
    text = ''
      #!${pkgs.zsh}/bin/zsh
      export PATH="/opt/homebrew/bin:$HOME/.nix-profile/bin:/etc/profiles/per-user/$USER/bin:$PATH"

       [[ -f "${localZsh}" ]] && source "${localZsh}"
       ${loadCoderCfApp}
       app="''${CODER_CF_APP:-}"
      if [[ -z "$app" ]]; then
        print -u2 "coder: CODER_CF_APP is not set"
        exit 1
      fi

      token="$(${pkgs.cloudflared}/bin/cloudflared access token -app "$app" 2>/dev/null)" || {
        print -u2 "coder: failed to obtain Cloudflare Access token for $app"
        exit 1
      }
      if [[ -z "$token" ]]; then
        print -u2 "coder: Cloudflare Access token is empty for $app"
        exit 1
      fi
      print -r -- "CF-Access-Token=$token"
    '';
  };

  coderHeaderCommand = "${coderCfHeader}/bin/coder-cf-header";
  coderBin = "/opt/homebrew/bin/coder";
  coderProxyCommand = "/usr/bin/env CODER_HEADER_COMMAND=${coderHeaderCommand} ${coderBin} ssh --stdio --ssh-host-prefix coder.";
in
{
  home.packages = [ coderCfHeader ];

  # Coder natively executes this command for every HTTP client. The session
  # token remains in macOS Keychain, while the short-lived Cloudflare token is
  # generated on demand and never stored in the environment or a file.
  home.sessionVariables.CODER_HEADER_COMMAND = coderHeaderCommand;

  # Keep `coder login` usable without manually exporting the deployment URL.
  # The URL is stored by Coder after the first successful login.
  programs.zsh.initContent = lib.mkAfter ''
    [[ -f "${localZsh}" ]] && source "${localZsh}"
    ${loadCoderCfApp}
    [[ -n "''${CODER_URL:-}" ]] || export CODER_URL="''${CODER_CF_APP:-}"
    export CODER_HEADER_COMMAND="${coderHeaderCommand}"
  '';

  programs.ssh.settings = {
    "coder.*" = {
      ConnectTimeout = 0;
      StrictHostKeyChecking = "accept-new";
      UserKnownHostsFile = "~/.ssh/known_hosts.coder";
      LogLevel = "ERROR";
      ProxyCommand = "${coderProxyCommand} %h";
    };
    "*.coder" = {
      ConnectTimeout = 0;
      StrictHostKeyChecking = "accept-new";
      UserKnownHostsFile = "~/.ssh/known_hosts.coder";
      LogLevel = "ERROR";
      ProxyCommand = "${coderProxyCommand} %h";
    };
  };
}
