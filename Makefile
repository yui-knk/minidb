BASE_DIR=/tmp/minidb
DBNAME=db1
TABLENAME=table1

.PHONY: init_db create_db create_table

init_db:
	cargo run -- --base_dir $(BASE_DIR) init

create_db:
	cargo run -- --base_dir $(BASE_DIR) create_db $(DBNAME)

create_table:
	cargo run -- --base_dir $(BASE_DIR) create_table $(DBNAME) $(TABLENAME)
