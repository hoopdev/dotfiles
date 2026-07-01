# Re-export the standalone `dev` fleet-tool flake's packages as top-level
# outputs of this flake, so `nix build .#dev` (and `.#dev-tui` / `.#dev-zellij`)
# keep working from dotfiles after the Rust workspace was extracted to its own
# repo (`inputs.dev`, see flake.nix).
#
# `home/mac/dev.nix` consumes `inputs.dev.packages.*` directly for the actual
# install; this module only surfaces the same artifacts as flake packages.
# The `dev` input and this flake both target { x86_64-linux, aarch64-darwin },
# so `inputs.dev.packages.${system}` always resolves.
{ inputs, ... }:
{
  perSystem =
    { system, ... }:
    {
      packages = {
        inherit (inputs.dev.packages.${system}) dev dev-tui dev-zellij;
      };
    };
}
