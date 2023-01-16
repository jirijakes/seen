clean:
	rm ~/.local/share/seen/seen.db
	rm -rf ~/.local/share/seen/index/
	sqlx db create
	sqlx migrate run
