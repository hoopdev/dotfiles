{ pkgs, ... }:
let
  # 1Password SSH agent socket path (varies by platform).
  # macOS:   the 1Password.app group container
  # Linux:   the 1Password GUI's agent (requires "Use the SSH agent" toggled
  #          on in 1Password > Settings > Developer)
  # WSL:     not handled here — bridge to Windows 1Password via npiperelay
  #          if needed.
  agentSocket =
    if pkgs.stdenv.isDarwin then
      "~/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"
    else
      "~/.1password/agent.sock";
in
{
  programs.ssh = {
    enable = true;
    # Opt out of home-manager's built-in `Host *` defaults (now deprecated) and
    # rely on OpenSSH's own defaults — we only need the 1Password agent here.
    enableDefaultConfig = false;
    # `settings` replaces the deprecated `matchBlocks`: the attribute name is the
    # `Host` pattern and keys are raw OpenSSH directives (IdentityAgent, …).
    settings."github.com gitlab.com bitbucket.org" = {
      IdentityAgent = ''"${agentSocket}"'';
    };
  };
}
