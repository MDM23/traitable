use crate::Message;

#[derive(Debug)]
pub struct User {
    pub id: usize,
    pub name: String,
}

#[derive(Debug)]
pub struct GetSelf {}

impl Message for GetSelf {
    type Response = User;
}

#[derive(Debug)]
pub struct SearchUsers {
    pub query: String,
    pub page: usize,
    pub per_page: usize,
}

impl Message for SearchUsers {
    type Response = Vec<User>;
}
