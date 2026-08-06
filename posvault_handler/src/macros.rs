#[macro_export]
macro_rules! ensure {
    ($cond:expr, $msg:literal $(,)?) => {
        if !$cond {
            return Err($crate::errors::PosVaultError::InvalidInput($msg.into()));
        }
    };
    ($cond:expr, $err:expr $(,)?) => {
        if !$cond {
            return Err($crate::errors::PosVaultError::InvalidInput($err.into()));
        }
    };
}

#[macro_export]
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err($crate::errors::PosVaultError::InvalidInput($msg.into()));
    };
    ($err:expr $(,)?) => {
        return Err($err.into());
    };
}
