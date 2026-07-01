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

      # Lean Rust toolchain for the pkgs/ Cargo workspace (dev-core / dev-cli /
      # dev-tui / dev-zellij). Deliberately separate from `default`: no zsh exec
      # and no Python cruft, so `nix develop .#rust -c cargo <cmd>` runs
      # non-interactively — the one-liner a coding agent (or CI) uses to verify
      # builds/tests without the cmake + libiconv + LIBRARY_PATH incantation.
      # See pkgs/CLAUDE.md and pkgs/justfile.
      devShells.rust = pkgs.mkShell {
        packages =
          with pkgs;
          [
            cargo
            rustc
            clippy
            rustfmt
            cmake # dev-core builds vendored libgit2 from source
            pkg-config
            just
          ]
          ++ lib.optionals stdenv.isDarwin [ libiconv ] # final link needs -liconv
          ++ lib.optionals stdenv.isLinux [ openssl ];
        # macOS: point the linker at libiconv so `cc … -liconv` resolves.
        shellHook = pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
          export LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ pkgs.libiconv ]}''${LIBRARY_PATH:+:$LIBRARY_PATH}"
        '';
      };
    };
}
