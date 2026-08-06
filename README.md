![GitHub Release](https://img.shields.io/github/v/release/tumrin/hyprland-focused-booster)
![AUR Version](https://img.shields.io/aur/version/hyprland-focused-booster)

# Hyprland-focused-booster

VRAM prioritization for Hyprland using dmemcg-booster based on
https://pixelcluster.github.io/VRAM-Mgmt-fixed/ and inspired by
https://github.com/1Naim/niri-focused-booster.

## Requirements

### Kernel

You'll need one of these kernel versions:

- linux-cachyos
- [linux-dmemcg](https://aur.archlinux.org/packages/linux-dmemcg)
- Linux kernel 7.3+

### dmemcg-booster

- [dmemcg-booster](https://aur.archlinux.org/packages/dmemcg-booster)

Remember to start the dmemcg-booster systemd service.

### Running apps as systemd units

You also need to use [runapp](https://github.com/c4rlo/runapp) or similar tool
to launch applications as systemd units for this to work properly.

For games you could launch each game as systemd unit too but this requires you
to set launch arguments for separately for every game. Launching Steam, Heroic,
etc. as systemd unit via runapp or similar tool should work well enough as VRAM
is prioritized to closest systemd cgroup and all it's subprocesses including the
active game window.

## Installation

### AUR

https://aur.archlinux.org/packages/hyprland-focused-booster

```bash
paru -S hyprland-focused-booster
```

### Manual

```bash
cargo build --release
cp target/release/hyprland-focused-booster /usr/bin/hyprland-focused-booster
cp hyprland-focused-booster.service /usr/lib/systemd/user/hyprland-focused-booster.service

systemctl --user enable --now hyprland-focused-booster.service # For current user
# OR
sudo systemctl --user --global enable hyprland-focused-booster.service # For all users
```
