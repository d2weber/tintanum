install: install_bin install_daemon start_daemon

install_bin:
	cargo install --path . --example dtintanum --root ~/.local/

install_daemon:
	cp examples/dtintanum/tintanum.service ~/.config/systemd/user/
	systemctl --user daemon-reload

start_daemon:
	systemctl --user enable tintanum
	systemctl --user start tintanum

uninstall: stop_daemon uninstall_daemon
	cargo uninstall tintanum --root ~/.local/

uninstall_daemon:
	rm ~/.config/systemd/user/tintanum.service
	systemctl --user daemon-reload


stop_daemon:
	systemctl --user stop tintanum
	systemctl --user disable tintanum
