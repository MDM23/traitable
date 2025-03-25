use serde::{Deserialize, Serialize};

use crate::AccessControl;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: usize,
    pub name: String,
    pub group_ids: Vec<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum UserPermission {
    Create,
    Delete,
    View,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum UserScope {
    Group(usize),
}

impl AccessControl for User {
    type Scope = UserScope;
    type Permission = UserPermission;

    fn within_scope(&self, scope: &Self::Scope) -> bool {
        match scope {
            UserScope::Group(id) => self.group_ids.contains(id),
        }
    }
}
