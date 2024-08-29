mod contacts;
mod users;

pub trait Message {
    type Response;
}

traitable::generate! {
    (Message) => {
        #[derive(Debug)]
        pub enum Request {
            $( $[R $index] ($ty_full), )*
        }

        $(
            impl Into<Request> for $ty_full {
                fn into(self) -> Request {
                    Request::$[R $index](self)
                }
            }
        )*
    }
}

#[test]
fn test() {
    let _: Request = contacts::AddContact {
        name: String::from("John Doe"),
    }
    .into();
}
