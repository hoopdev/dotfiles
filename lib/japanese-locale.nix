_:

{
  # Timezone
  time.timeZone = "Asia/Tokyo";

  # Japanese locale
  i18n.defaultLocale = "ja_JP.UTF-8";
  i18n.extraLocaleSettings = {
    LC_ADDRESS = "ja_JP.UTF-8";
    LC_IDENTIFICATION = "ja_JP.UTF-8";
    LC_MEASUREMENT = "ja_JP.UTF-8";
    LC_MONETARY = "ja_JP.UTF-8";
    LC_NAME = "ja_JP.UTF-8";
    LC_NUMERIC = "ja_JP.UTF-8";
    LC_PAPER = "ja_JP.UTF-8";
    LC_TELEPHONE = "ja_JP.UTF-8";
    LC_TIME = "ja_JP.UTF-8";
  };

  # The input method (Fcitx5 + Mozc) is desktop-only and lives in
  # lib/japanese-input.nix; headless hosts import just this file.

  # Fonts themselves (HackGen + Noto CJK + emoji) are installed and named by
  # Stylix — see lib/stylix.nix. Stylix's fontconfig target already puts each
  # family's font first in defaultFonts, so all that is added here is the emoji
  # fallback it does not append. Listing the fonts again would only duplicate
  # every entry.
  fonts = {
    fontDir.enable = true;
    fontconfig.defaultFonts = {
      serif = [ "Noto Color Emoji" ];
      sansSerif = [ "Noto Color Emoji" ];
      monospace = [ "Noto Color Emoji" ];
    };
  };
}
