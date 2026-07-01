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

## Rust workspace (`pkgs/`)

`dev-core` / `dev-cli` / `dev-tui` / `dev-zellij` — see
[dev-rust-workspace.md](dev-rust-workspace.md).

```bash
nix develop .#rust -c just -f pkgs/justfile ci   # cargo check + test
nix develop .#rust -c cargo <cmd> ...            # lean Rust toolchain shell
```
