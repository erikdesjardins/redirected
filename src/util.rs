use failure::Fail;

#[derive(Debug, Fail)]
#[fail(display = "Option was None")]
pub struct NoneError(());

pub trait OptionExt {
    type Out;
    fn into_result(self) -> Result<Self::Out, NoneError>;
}

impl<T> OptionExt for Option<T> {
    type Out = T;
    fn into_result(self) -> Result<Self::Out, NoneError> {
        match self {
            Some(x) => Ok(x),
            None => Err(NoneError(())),
        }
    }
}
