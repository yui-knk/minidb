BASE_DIR=/tmp/minidb
DBNAME=db1
TABLENAME=table1

.PHONY: setup test_run init_db create_db create_table insert_into insert_into2 select_from

setup: clean init_db create_db create_table

test_run: setup insert_into select_from

clean:
	rm -rf $(BASE_DIR)

init_db:
	cargo run -- --base_dir $(BASE_DIR) init

create_db:
	cargo run -- --base_dir $(BASE_DIR) create_db $(DBNAME)

create_table:
	cargo run -- --base_dir $(BASE_DIR) create_table $(DBNAME) $(TABLENAME)

insert_into:
	cargo run -- --base_dir $(BASE_DIR) insert_into $(DBNAME) $(TABLENAME) 1 12
	cargo run -- --base_dir $(BASE_DIR) insert_into $(DBNAME) $(TABLENAME) 2 13
	cargo run -- --base_dir $(BASE_DIR) insert_into $(DBNAME) $(TABLENAME) 3 12
	cargo run -- --base_dir $(BASE_DIR) insert_into $(DBNAME) $(TABLENAME) 4 20
	cargo run -- --base_dir $(BASE_DIR) insert_into $(DBNAME) $(TABLENAME) 5 21

insert_into2: insert_into insert_into insert_into insert_into insert_into

select_from:
	cargo run -- --base_dir $(BASE_DIR) select_from $(DBNAME) $(TABLENAME) 1 12
