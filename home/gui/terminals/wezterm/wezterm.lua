local wezterm = require("wezterm")

return {
	color_scheme = "KT-Theme",
	font = wezterm.font_with_fallback({
		{ family = "HackGen Console NF", weight = "Regular" },
		{ family = "HackGen Console NF", weight = "Regular", assume_emoji_presentation = true },
		{ family = "Noto Sans CJK JP" },
	}),
	warn_about_missing_glyphs = false,
	window_frame = {
		font_size = 10.0,
	},
	window_padding = {
		left = 10,
		right = 10,
		top = 5,
		bottom = 5,
	},
	use_fancy_tab_bar = false,
	hide_tab_bar_if_only_one_tab = true,
	use_ime = true, -- Enable IME
	check_for_updates = false, -- Disable update check
	audible_bell = "Disabled", -- Disable bell
}