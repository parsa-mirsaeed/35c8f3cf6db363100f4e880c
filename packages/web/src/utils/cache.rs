use dioxus::prelude::*;
use api::server_functions::user_management::{UserListItem, UserStats};
use api::server_functions::class_functions::ClassSectionResponse;
use api::models::Subject;

#[derive(Clone, Debug, PartialEq)]
pub struct UserFilters {
    pub role: String,
    pub status: String,
    pub query: String,
}

#[derive(Clone, Copy)]
pub struct AppCache {
    pub users: Signal<Option<(Vec<UserListItem>, UserFilters)>>,
    pub user_stats: Signal<Option<UserStats>>,
    pub classes: Signal<Option<Vec<ClassSectionResponse>>>,
    pub subjects: Signal<Option<Vec<Subject>>>,
}

impl AppCache {
    pub fn new() -> Self {
        Self {
            users: Signal::new(None),
            user_stats: Signal::new(None),
            classes: Signal::new(None),
            subjects: Signal::new(None),
        }
    }

    pub fn invalidate_users(&mut self) {
        self.users.set(None);
        self.user_stats.set(None);
    }

    pub fn invalidate_classes(&mut self) {
        self.classes.set(None);
    }

    pub fn invalidate_subjects(&mut self) {
        self.subjects.set(None);
    }
}

pub fn use_app_cache() -> AppCache {
    use_context::<AppCache>()
}

pub fn init_app_cache() -> AppCache {
    use_context_provider(|| AppCache::new())
}
