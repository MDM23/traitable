use crate::Message;

#[derive(Debug)]
pub enum ContactType {
    Internal,
    External,
}

#[derive(Debug)]
pub struct Contact {
    pub id: usize,
    pub user_id: usize,
    pub ty: ContactType,
    pub name: String,
}

#[derive(Debug)]
pub struct GetContacts {
    pub user_id: usize,
}

impl Message for GetContacts {
    type Response = Vec<Contact>;
}

#[derive(Debug)]
pub struct AddContact {
    pub name: String,
}

impl Message for AddContact {
    type Response = Result<Contact, String>;
}
