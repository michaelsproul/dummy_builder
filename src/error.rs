#[derive(Debug)]
pub enum Error {
    /// HTTP 204 for no payload.
    NoPayload,
    /// HTTP 500 for errors that shouldn't occur.
    LogicError,
}
