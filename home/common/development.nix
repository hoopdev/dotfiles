# Development and AI tooling is opt-in rather than part of every CLI profile.
{
  imports = [
    ./cli/development.nix
    ./cli/claude-code.nix
    ./cli/opencode.nix
  ];
}
