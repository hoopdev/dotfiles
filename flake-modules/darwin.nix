{ inputs, helpers, ... }:
let
  inherit (inputs.nixpkgs) lib;
  inherit (helpers)
    defaultUsername
    nixpkgsConfig
    mkHomeConfiguration
    hosts
    ;

  darwinHosts = lib.filterAttrs (_: meta: meta.type == "darwin") hosts;

  # If meta sets `configFrom = "x"`, configuration/home are picked up from
  # hosts/x instead of the host's own directory (used for shared mac config).
  hostDir = name: meta: ../hosts/${meta.configFrom or name};

  mkDarwinConfiguration =
    hostname: meta:
    inputs.nix-darwin.lib.darwinSystem {
      system = "aarch64-darwin";
      modules = [
        { nixpkgs.config = nixpkgsConfig; }
        # Workaround: nixpkgs-unstable's nixos-render-docs dropped the
        # `--toc-depth` flag (now `--sidebar-depth`), but the current
        # nix-darwin release still passes it when building the HTML manual,
        # breaking `darwin-manual-html` and thus every rebuild. Two paths pull
        # it in, so both are cut until nix-darwin catches up (man pages use a
        # different renderer and stay enabled):
        #   - doc.enable: our own system's HTML manual + `darwin-help` command.
        #   - darwin-uninstaller: bundles its own default-config darwin-system,
        #     which rebuilds the manual and ignores the override above.
        # Remove both once upstream syncs.
        {
          documentation.doc.enable = false;
          system.tools.darwin-uninstaller.enable = false;
        }
        (hostDir hostname meta + "/configuration.nix")
        inputs.stylix.darwinModules.stylix
        (import ../lib/stylix.nix { darwin = true; })
        inputs.home-manager.darwinModules.home-manager
        (mkHomeConfiguration {
          username = defaultUsername;
          inherit hostname;
          hostPath = hostDir hostname meta + "/home.nix";
        })
      ];
      specialArgs = {
        inherit inputs;
        inherit (inputs.nixpkgs) lib;
      };
    };
in
{
  flake.darwinConfigurations = lib.mapAttrs mkDarwinConfiguration darwinHosts;
}
