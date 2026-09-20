# Development and AI tooling is opt-in rather than part of every CLI profile.
{
  imports = [
    ./cli/development.nix
    ./cli/ai-tools.nix
    ./cli/claude-code.nix
    ./cli/opencode.nix
  ];

  programs.direnv = {
    enable = true;
    enableZshIntegration = true;
    enableNushellIntegration = true;
    nix-direnv.enable = true;
  };
}
