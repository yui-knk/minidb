BASE_DIR=/tmp/minidb
DBNAME=db1
TABLENAME=table1

.PHONY: init_db create_db create_table insert_into

init_db:
	cargo run -- --base_dir $(BASE_DIR) init

create_db:
	cargo run -- --base_dir $(BASE_DIR) create_db $(DBNAME)

create_table:
	cargo run -- --base_dir $(BASE_DIR) create_table $(DBNAME) $(TABLENAME)

insert_into:
	cargo run -- --base_dir $(BASE_DIR) insert_into $(DBNAME) $(TABLENAME) 1 12
