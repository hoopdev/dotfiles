# 1Password CLI + GUI with polkit policy for ktaga.
_:

{
  programs._1password.enable = true;
  programs._1password-gui = {
    enable = true;
    polkitPolicyOwners = [ "ktaga" ];
  };
}
