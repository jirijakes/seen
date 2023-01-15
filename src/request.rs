use crate::extract::Extract;

pub struct Request {
    readability: Box<dyn Extract>,
}
