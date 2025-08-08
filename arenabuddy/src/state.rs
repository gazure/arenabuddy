#[derive(Clone, Debug)]
pub(crate) enum AsyncState<T, E = String> {
    Loading,
    Error(E),
    Success(T),
}

impl<T, E> AsyncState<T, E> {
    pub fn is_loading(&self) -> bool {
        matches!(self, AsyncState::Loading)
    }

    pub fn details(&self) -> Option<&T> {
        match self {
            AsyncState::Success(details) => Some(details),
            _ => None,
        }
    }
}
