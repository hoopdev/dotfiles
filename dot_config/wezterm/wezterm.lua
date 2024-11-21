local wezterm = require("wezterm")

return {
    default_prog = {"nu"},
	color_scheme = "nord",
	window_background_opacity = 0.9,

	font = wezterm.font("Hack Nerd Font", { weight = "Regular", stretch = "Normal", style = "Normal" }),
	font_size = 14.0,

	window_padding = {
		left = 10,
		right = 10,
		top = 10,
		bottom = 10,
	},

	use_fancy_tab_bar = false,
	hide_tab_bar_if_only_one_tab = true,
	window_decorations = "RESIZE",

	front_end = "WebGpu",
	-- use_ime = true,
	check_for_updates = false,
    leader = { key="t", mods="CTRL" },
    disable_default_key_bindings = true,
    keys = {
        { key = "t", mods = "LEADER|CTRL",  action=wezterm.action{SendString="\x01"}},
        { key = "-", mods = "LEADER",       action=wezterm.action{SplitVertical={domain="CurrentPaneDomain"}}},
        { key = "\\",mods = "LEADER",       action=wezterm.action{SplitHorizontal={domain="CurrentPaneDomain"}}},
        { key = "z", mods = "LEADER",       action="TogglePaneZoomState" },
        { key = "c", mods = "LEADER",       action=wezterm.action{SpawnTab="CurrentPaneDomain"}},
        { key = "h", mods = "LEADER",       action=wezterm.action{ActivatePaneDirection="Left"}},
        { key = "j", mods = "LEADER",       action=wezterm.action{ActivatePaneDirection="Down"}},
        { key = "k", mods = "LEADER",       action=wezterm.action{ActivatePaneDirection="Up"}},
        { key = "l", mods = "LEADER",       action=wezterm.action{ActivatePaneDirection="Right"}},
        { key = "H", mods = "LEADER|SHIFT", action=wezterm.action{AdjustPaneSize={"Left", 5}}},
        { key = "J", mods = "LEADER|SHIFT", action=wezterm.action{AdjustPaneSize={"Down", 5}}},
        { key = "K", mods = "LEADER|SHIFT", action=wezterm.action{AdjustPaneSize={"Up", 5}}},
        { key = "L", mods = "LEADER|SHIFT", action=wezterm.action{AdjustPaneSize={"Right", 5}}},
        { key = "1", mods = "LEADER",       action=wezterm.action{ActivateTab=0}},
        { key = "2", mods = "LEADER",       action=wezterm.action{ActivateTab=1}},
        { key = "3", mods = "LEADER",       action=wezterm.action{ActivateTab=2}},
        { key = "4", mods = "LEADER",       action=wezterm.action{ActivateTab=3}},
        { key = "5", mods = "LEADER",       action=wezterm.action{ActivateTab=4}},
        { key = "6", mods = "LEADER",       action=wezterm.action{ActivateTab=5}},
        { key = "7", mods = "LEADER",       action=wezterm.action{ActivateTab=6}},
        { key = "8", mods = "LEADER",       action=wezterm.action{ActivateTab=7}},
        { key = "9", mods = "LEADER",       action=wezterm.action{ActivateTab=8}},
        { key = "&", mods = "LEADER|SHIFT", action=wezterm.action{CloseCurrentTab={confirm=true}}},
        { key = "x", mods = "LEADER",       action=wezterm.action{CloseCurrentPane={confirm=true}}},

        { key = "n", mods="SHIFT|CTRL",     action="ToggleFullScreen" },
        { key ="v",  mods="SHIFT|CTRL",    action=wezterm.action.PasteFrom 'Clipboard'},
        { key ="c",  mods="SHIFT|CTRL",    action=wezterm.action.CopyTo 'Clipboard'},
        { key = "+", mods="SHIFT|CTRL",     action="IncreaseFontSize" },
        { key = "-", mods="SHIFT|CTRL",     action="DecreaseFontSize" },
        { key = "0", mods="SHIFT|CTRL",     action="ResetFontSize" },
    },
}
