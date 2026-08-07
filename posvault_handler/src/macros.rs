#[macro_export]
macro_rules! ensure {
    ($cond:expr, $msg:literal $(,)?) => {
        if !$cond {
            return Err($crate::errors::PosVaultError::InvalidInput($msg.into()));
        }
    };
}

#[macro_export]
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err($crate::errors::PosVaultError::InvalidInput($msg.into()));
    };
}
