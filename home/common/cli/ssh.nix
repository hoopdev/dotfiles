{
  programs.ssh = {
    enable = true;
    # Opt out of home-manager's built-in `Host *` defaults (now deprecated) and
    # rely on OpenSSH's own defaults.
    enableDefaultConfig = false;
    # Pull in a git-ignored, machine-local config for private hosts (hostnames,
    # ForwardAgent for personal boxes). Kept out of this repo so no private host
    # info is committed. Resolved relative to ~/.ssh; missing file is ignored by
    # OpenSSH.
    includes = [ "config.local" ];
    # IdentityAgent is intentionally absent from every Host block.
    # $SSH_AUTH_SOCK is the single source of truth for which agent to use:
    #   - macOS local login: home/mac/default.nix loginExtra sets SSH_AUTH_SOCK
    #                        to the 1Password agent socket.
    #   - SSH session with ForwardAgent: sshd injects the caller's forwarded
    #                        agent into SSH_AUTH_SOCK; zsh init does not
    #                        fall back to the remote machine's local agent.
    # Setting IdentityAgent would pin a specific socket and break the second case.
  };
}
