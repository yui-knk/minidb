// This is for most everything that has columns or is otherwise
// similar to a table. For example "table".

const NAME: &'static str = "mini_class";

struct Record {
    name: String,
}

struct MiniClass {
}

impl MiniClass {

    pub fn name(&self) -> &str {
        &NAME
    }
}
