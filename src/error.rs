use eth2::types::Hash256;

#[derive(Debug)]
pub enum Error {
    /// HTTP 204 for no payload.
    NoPayload,
    /// HTTP 500 for unable to unbind header.
    UnbindPayloadError(Hash256),
    /// HTTP 500 for errors that shouldn't occur.
    LogicError,
}
