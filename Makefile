clean:
	rm seen.db
	rm -rf index/
	sqlx db create
	sqlx migrate run
