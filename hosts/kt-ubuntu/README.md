# kt-ubuntu Configuration

Standalone home-manager configuration for Ubuntu systems (non-NixOS).

This configuration includes:
- **WezTerm** terminal with Nerd Fonts pre-configured
- **Starship** prompt with Ubuntu logo (󰕈) and icons
- **Fonts** automatically installed (Nerd Fonts, Noto CJK, Emoji)

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

**WezTerm** is automatically configured with JetBrainsMono Nerd Font Mono.

For other terminals, set font to:
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

- **Terminal**: WezTerm with JetBrainsMono Nerd Font Mono
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

1. **Use WezTerm**: WezTerm is pre-configured with Nerd Fonts
   ```bash
   wezterm
   ```

2. **For other terminals**: Ensure Nerd Font is properly configured
   - Set font to `JetBrainsMono Nerd Font Mono`
   - Restart terminal after font changes

3. Test Unicode support:
   ```bash
   echo "󰕈"  # Should display Ubuntu logo
   ```

### Using WezTerm in code-server

If you're using code-server (VS Code in browser):

1. **SSH with X11 forwarding**:
   ```bash
   ssh -X user@kt-ubuntu
   wezterm
   ```

2. **Local terminal**: Connect via SSH and run WezTerm locally
   ```bash
   ssh user@kt-ubuntu
   wezterm
   ```

3. WezTerm will display with proper Nerd Font icons and Ubuntu logo

## Notes

- This configuration is for **standalone home-manager** only (not NixOS)
- Fonts are managed by home-manager and installed to `~/.nix-profile/share/fonts`
- Nerd Fonts are automatically installed for proper icon display in Starship
- After first install, you may need to restart your terminal or run `fc-cache -fv`
