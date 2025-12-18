# kt-ubuntu Configuration

Standalone home-manager configuration for Ubuntu systems (non-NixOS).

This configuration includes:
- **Starship** prompt with Ubuntu logo (󰕈) and icons
- **Fonts** automatically installed (Nerd Fonts, Noto CJK, Emoji)
- **Shell**: Zsh and Nushell with consistent configuration

## Prerequisites

### 1. Install Nix

```bash
sh <(curl -L https://nixos.org/nix/install) --daemon
```

### 2. Install Home Manager

```bash
nix-channel --add https://github.com/nix-community/home-manager/archive/master.tar.gz home-manager
nix-channel --update
nix-shell '<home-manager>' -A install
```

### 3. Enable Nix Flakes

```bash
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" > ~/.config/nix/nix.conf
```

### 4. Fonts (Managed by Home Manager)

Fonts are automatically installed by home-manager:
- **Noto Fonts CJK Sans**: For Japanese and CJK characters
- **Noto Color Emoji**: For emoji support
- **Nerd Fonts**: FiraCode, JetBrainsMono, and Meslo for icons

**Configure your terminal** to use a Nerd Font for proper icon display:
- `JetBrainsMono Nerd Font Mono` (recommended)
- `FiraCode Nerd Font Mono`
- `MesloLGS Nerd Font Mono`

## Usage

### Apply Configuration

For user `jovyan`:
```bash
nix run home-manager/master -- switch --flake .#jovyan@kt-ubuntu
```

For user `ktaga`:
```bash
nix run home-manager/master -- switch --flake .#ktaga@kt-ubuntu
```

After first install, update font cache:
```bash
fc-cache -fv
```

### Update Flake Inputs

```bash
nix flake update
```

## Features

- **Shell**: Zsh and Nushell with Starship prompt
- **Starship**: Custom prompt with Ubuntu logo (󰕈) and icons
- **Editor**: Neovim (nixvim configuration)
- **Git**: Pre-configured with common aliases
- **Color Scheme**: Nord theme via nix-colors
- **Development Tools**: Python, uv, Nix tools
- **Fonts**: Nerd Fonts (FiraCode, JetBrainsMono, Meslo), Noto CJK, Emoji

## Troubleshooting

### Icons not displaying

1. Verify Nerd Font is installed by home-manager:
   ```bash
   fc-list | grep -i "nerd"
   ```

2. Check terminal font configuration:
   - Ensure terminal is set to use a Nerd Font (e.g., "FiraCode Nerd Font Mono")
   - Restart terminal after font changes

3. If fonts are not found, rebuild font cache:
   ```bash
   fc-cache -fv
   ```

### Starship not showing Ubuntu logo

1. **Configure terminal font**: Ensure your terminal uses a Nerd Font
   - Set font to `JetBrainsMono Nerd Font Mono` (recommended)
   - Or use `FiraCode Nerd Font Mono` or `MesloLGS Nerd Font Mono`
   - Restart terminal after font changes

2. **Test Unicode support**:
   ```bash
   echo "󰕈"  # Should display Ubuntu logo
   ```

3. **Recommended terminals with Nerd Font support**:
   - **Alacritty**: Fast GPU-accelerated terminal
   - **Kitty**: Feature-rich with image support
   - **GNOME Terminal**: Default Ubuntu terminal
   - **Konsole**: KDE terminal
   - **Tilix**: Tiling terminal emulator

4. **For code-server/JupyterLab**:
   The browser-based terminal may not support Nerd Fonts properly.
   Use a native terminal application for best results.

## Notes

- This configuration is for **standalone home-manager** only (not NixOS)
- Fonts are managed by home-manager and installed to `~/.nix-profile/share/fonts`
- Nerd Fonts are automatically installed for proper icon display in Starship
- After first install, you may need to restart your terminal or run `fc-cache -fv`
