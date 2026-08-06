use crate::errors::Result;

pub trait Validate {
    fn validate(&self) -> Result<()>;
}
