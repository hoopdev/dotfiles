# 1Password is optional and its policy owners are host metadata, never a
# hard-coded username.
{
  config,
  lib,
  ...
}:
let
  cfg = config.dotfiles.onepassword;
in
{
  options.dotfiles.onepassword = {
    enable = lib.mkEnableOption "1Password CLI and GUI integration";
    polkitPolicyOwners = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = "Users allowed to manage the 1Password GUI polkit policy.";
    };
  };

  config = lib.mkIf cfg.enable {
    programs._1password.enable = true;
    programs._1password-gui = {
      enable = true;
      inherit (cfg) polkitPolicyOwners;
    };
  };
}
