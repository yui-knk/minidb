BASE_DIR=/tmp/minidb
DBNAME=db1
TABLENAME=table1
LOGLEVEL=warn

.PHONY: setup test_run init_db create_db create_table insert_into insert_into2 select_from count

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
	cargo run -- --base_dir $(BASE_DIR) execute "insert into $(DBNAME).$(TABLENAME) (id, age) values (1, 12), (2, 13), (3, 12), (4, 20), (5, 21)"

insert_into2: insert_into insert_into insert_into insert_into insert_into

select_from:
	cargo run -- --base_dir $(BASE_DIR) --log_level $(LOGLEVEL) execute "select * from $(DBNAME).$(TABLENAME)"

count:
	cargo run -- --base_dir $(BASE_DIR) --log_level $(LOGLEVEL) execute "select count() from $(DBNAME).$(TABLENAME)"

delete:
	cargo run -- --base_dir $(BASE_DIR) --log_level $(LOGLEVEL) execute "delete from $(DBNAME).$(TABLENAME)"
