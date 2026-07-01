{
  pkgs,
  config,
  ...
}:
let
  coderSessionPath = "${config.home.homeDirectory}/Library/Application Support/coderv2/session";
  # local.zsh (untracked) holds only the secrets these wrappers source:
  # CODER_URL / CODER_CF_APP for the cloudflared-fronted proxy.
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

  coderBin = "/opt/homebrew/bin/coder";

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
in
{
  # Coder environment connection ONLY: the cloudflared-fronted proxy/CLI wrappers
  # and the SSH routing for coder hosts. Everything else that used to live here
  # moved to its own module — the `dev` fleet tool + opencode + the Claude Code
  # statusline (`dev statusline`) all live in dev.nix.
  home.packages = [
    coderProxy
    coderCli
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
