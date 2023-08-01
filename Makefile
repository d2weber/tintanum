install: install_bin install_daemon start_daemon

uninstall: stop_daemon
	rm ~/.local/bin/color-scheme-hook
	rm ~/.config/systemd/user/color-scheme-preference-hook.service

install_bin:
	cargo build --release
	systemctl --user stop color-scheme-preference-hook || true
	cp target/release/color-scheme-hook ~/.local/bin/

install_daemon:
	systemctl --user daemon-reload
	cp color-scheme-preference-hook.service ~/.config/systemd/user/

start_daemon:
	systemctl --user enable color-scheme-preference-hook
	systemctl --user start color-scheme-preference-hook

stop_daemon:
	systemctl --user stop color-scheme-preference-hook
	systemctl --user disable color-scheme-preference-hook
