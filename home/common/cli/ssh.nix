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
    #   - SSH session with ForwardAgent: sshd overwrites SSH_AUTH_SOCK with the
    #                        caller's forwarded agent; loginExtra is skipped
    #                        (SSH_CLIENT / SSH_TTY are set) so the caller's keys
    #                        are used transparently.
    # Setting IdentityAgent would pin a specific socket and break the second case.
  };
}
