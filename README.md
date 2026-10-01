![GitHub Release](https://img.shields.io/github/v/release/tumrin/hyprland-focused-booster)
![AUR Version](https://img.shields.io/aur/version/hyprland-focused-booster)

# Hyprland-focused-booster

VRAM, CPU and RAM prioritization for Hyprland using cgroups.

VRAM prioritization is done using dmemcg-booster based on
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

## Configuration

Configuration is optional and default values will be used if configuration file
does not exist.

`$XDG_CONFIG_HOME/hyprland-focused-booster.toml` (usually
`~/.config/hyprland-focused-booster.toml`). Missing file or keys fall back to
defaults below. Restart the service after editing
(`systemctl --user restart hyprland-focused-booster.service`). Out-of-range
values and malformed files cause default values to be used.

### Default values

To keep same behavior as 0.1 only VRAM boosting is enabled by default.
Furthermore CPU and RAM boosting are considered experimental as I have not yet
tested them extensively. These might be enabled by default in the future or
dropped entirely.

```toml
vram_boost = true # Enable VRAM prioritization for focused application
vram_boost_value = 100.0 # Percentage of VRAM priorized for focused application. Values between 0.0 and 100.0 (inclusive)
cpu_boost = false # Enable CPU prioritization for focused application
cpu_boost_value = 10000 # CPU weight value. Higher means more prioritization. Values between 100 and 10000 (inclusive)
ram_boost = false # Enable RAM prioritization for focused application.
ram_boost_value = 100.0 # Percentage of RAM prioritized for focused application. Values between 0.0 and 100.0 (inclusive)
```
