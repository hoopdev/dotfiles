{ inputs, helpers, ... }:
{
  imports = [ inputs.treefmt-nix.flakeModule ];

  perSystem =
    { pkgs, system, ... }:
    {
      # Re-import nixpkgs with the shared config (allowUnfree etc.) so that
      # devShell tooling pulling in unfree packages (e.g. unrar via uutils) eval.
      _module.args.pkgs = import inputs.nixpkgs {
        inherit system;
        config = helpers.nixpkgsConfig;
      };

      treefmt = {
        projectRootFile = "flake.nix";
        programs = {
          nixfmt.enable = true;
          deadnix.enable = true;
          statix.enable = true;
        };
      };

      devShells.default =
        let
          devshell = import ../lib/devshell.nix {
            inherit pkgs;
            inherit (inputs.nixpkgs) lib;
          };
        in
        devshell.shells.default { inherit devshell; };
    };
}
