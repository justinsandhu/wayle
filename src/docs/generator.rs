use super::{ModuleInfo, get_all_modules};

pub struct DocsGenerator {
    modules: Vec<ModuleInfo>,
}

impl DocsGenerator {
    pub fn new() -> Self {
        Self {
            modules: get_all_modules(),
        }
    }
}
