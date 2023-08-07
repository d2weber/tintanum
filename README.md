# Tintanum

This is a deamon that listens for a change of the color scheme preference and changes
config files accordingly. Currently supported:
 * [Alacritty](https://github.com/alacritty/alacritty)
 * [Helix](https://github.com/helix-editor/helix/)

### Dependencies

* cargo
* pkill


## Usage

Within the repository root run:

```bash
make install
```

### Configure themes

Specify your themes in the config:

```yaml
import:
 - ~/.config/alacritty/themes/themes/your-light-theme.yaml #[light]
# - ~/.config/alacritty/themes/themes/your-dark-theme.yaml #[dark]
```

As soon as the daemon runs it will adjust the commented lines according to the
system scheme preference.


## Uninstall

Uninstall systemd service

```bash
make uninstall
```
