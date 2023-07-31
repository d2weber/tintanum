# Color scheme hook

This is a deamon that listens for a change of the color scheme preference.
It switches the theme of alacritty accordingly.

## Usage

### Build deamon and setup systemd service

Within this repository run:

```bash
cargo build --release
cp target/release/color-scheme-hook ~/.local/bin/
cp color-scheme-preference-hook.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable color-scheme-preference-hook
systemctl --user start color-scheme-preference-hook
```

### Configure themes

* Add required import to your alacritty configuration. The `current_auto_theme.yml` will
be created automatically.

  ```bash
  import:
   - ~/.config/alacritty/current_auto_theme.yml
  ```

* Specify your desired themes in these files (potentially symlinked):
  * `~/.config/alacritty/theme-light.yml`
  * `~/.config/alacritty/theme-dark.yml`
