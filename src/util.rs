pub trait IntoOptionExt: Sized {
    fn some_if(self, f: impl FnOnce(&Self) -> bool) -> Option<Self> {
        if f(&self) {
            Some(self)
        } else {
            None
        }
    }
}

impl<T> IntoOptionExt for T {}
