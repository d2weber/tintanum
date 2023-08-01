# Color scheme hook

This is a deamon that listens for a change of the color scheme preference.
It switches the theme of alacritty accordingly.

### Dependencies

* cargo
* pkill


## Usage

Within the repository root run:

```bash
make install
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


## Uninstall

Undo the edits:
* Restore your alacritty configuration
* Remove `theme-light` and `dark`
* Uninstall service

  ```bash
  make uninstall
  ```
