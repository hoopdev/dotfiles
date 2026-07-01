# Commands

## Apply

`nh` is preferred (raw fallback: `sudo nixos-rebuild switch --flake .#<host>`).

```bash
nh os switch . -H <host>          # NixOS: kt-thinkpad, kt-proxmox, kt-wsl
nh darwin switch . -H <host>      # macOS: kt-mac-studio, kt-mac-mini, kt-mba
nh home switch . -c <user>@<host> # home-manager: ktaga@kt-ubuntu, jovyan@kt-ubuntu
```

## Bootstrap (fresh host, no nh yet)

```bash
nix run nixpkgs#nh -- darwin switch . -H kt-mac-studio
```

## Maintenance

```bash
nix flake update                        # Update inputs
nix flake check                         # Validate
nh clean all --keep 5 --keep-since 7d   # GC (user + system)
nix develop                             # Dev shell (Python + Nix tools)
```

## dev fleet tool (`~/git/dev`)

The `dev` Rust workspace (`dev-core` / `dev-cli` / `dev-tui` / `dev-zellij`) was
extracted to its own flake. dotfiles consumes it via the `dev` flake input and
installs it in `home/mac/dev.nix`; `flake-modules/dev.nix` also re-exports it.

```bash
# Build from dotfiles (uses the pinned input):
nix build .#dev                          # dev CLI (also .#dev-tui / .#dev-zellij)
nix flake update dev                      # pull latest dev into dotfiles' lock

# Iterate on the workspace itself (toolchain lives in the dev repo's `rust` shell):
cd ~/git/dev
nix develop .#rust -c just ci             # cargo check + test
nix develop .#rust -c cargo <cmd> ...     # lean Rust toolchain shell
```
