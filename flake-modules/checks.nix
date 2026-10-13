# `nix flake check` natively evaluates NixOS configurations, but merely seeing
# Darwin/Home Manager attribute sets is not enough to catch a broken host. These
# tiny text derivations force every configuration's activation/toplevel drvPath
# during evaluation without building a full system closure.
{ inputs, helpers, ... }:
let
  inherit (inputs.nixpkgs) lib;
  inherit (helpers) hosts;
in
{
  perSystem =
    { pkgs, system, ... }:
    let
      evalCheck = name: value: pkgs.writeText name (builtins.deepSeq value "ok\n");
      namedChecks =
        prefix: values:
        lib.listToAttrs (map (value: lib.nameValuePair "${prefix}-${value.name}" value.check) values);
      nixosChecks = namedChecks "eval-nixos" (
        lib.mapAttrsToList (name: _meta: {
          inherit name;
          check =
            evalCheck "eval-nixos-${name}"
              inputs.self.nixosConfigurations.${name}.config.system.build.toplevel.drvPath;
        }) (lib.filterAttrs (_: meta: meta.type == "nixos" && meta.system == system) hosts)
      );
      darwinChecks = namedChecks "eval-darwin" (
        lib.mapAttrsToList (name: _meta: {
          inherit name;
          check =
            evalCheck "eval-darwin-${name}"
              inputs.self.darwinConfigurations.${name}.config.system.build.toplevel.drvPath;
        }) (lib.filterAttrs (_: meta: meta.type == "darwin" && meta.system == system) hosts)
      );
      homeChecks = namedChecks "eval-home" (
        lib.concatMap (
          { name, value }:
          map (username: {
            name = "${username}-${name}";
            check =
              evalCheck "eval-home-${username}-${name}"
                inputs.self.homeConfigurations."${username}@${name}".activationPackage.drvPath;
          }) value.users
        ) (lib.attrsToList (lib.filterAttrs (_: meta: meta.type == "home" && meta.system == system) hosts))
      );
      secretsModule = import ../home/common/cli/onepassword.nix {
        inherit lib;
        pkgs = { };
        config.dotfiles.secrets.env = { };
      };
      secretReference = secretsModule.options.dotfiles.secrets.env.type.nestedTypes.elemType;
      secretReferenceCheck =
        assert secretReference.check "op://Personal/Example/credential";
        assert secretReference.check "op://Personal/Example/section/field";
        assert !(secretReference.check "plaintext-credential");
        assert !(secretReference.check "op://Personal/Example");
        assert !(secretReference.check "op://Personal/Example/field\nINJECTED=value");
        assert !(secretReference.check "op://Personal/Example/field\rINJECTED=value");
        "ok\n";
    in
    {
      checks =
        nixosChecks
        // darwinChecks
        // homeChecks
        // {
          secrets-reference-type = evalCheck "secrets-reference-type" secretReferenceCheck;
        };
    };
}
