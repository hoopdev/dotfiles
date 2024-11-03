local wezterm = require("wezterm")

return {
	color_scheme = "MyTheme",
	font = wezterm.font_with_fallback({
		{ family = "HackGen Console NF", weight = "Regular" },
		{ family = "HackGen Console NF", weight = "Regular", assume_emoji_presentation = true },
		{ family = "Noto Sans CJK JP" },
	}),
    font_size = 12.0,
	use_ime = true, -- Enable IME
	check_for_updates = false, -- Disable update check
}