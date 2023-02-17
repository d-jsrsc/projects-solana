use anchor_lang::prelude::*;

#[error_code]
pub enum Wen3ExError {
    #[msg("InvalidAmount take")]
    InvalidAmount,
    #[msg("Current owner is not the authority of the parent token")]
    InvalidAuthority,
    #[msg("Only Reversible Synthetic Tokens can be extracted")]
    InvalidExtractAttempt,
    #[msg("Wrong type of burn instruction for the token")]
    InvalidBurnType,
    #[msg("Wrong opration of crank process instruction for the token")]
    InvalidTransferCrankProcess,
    #[msg("NoTakerTokenAccount")]
    NoTakerTokenAccount,
    #[msg("IncorrectTakerTokenAccount")]
    IncorrectTakerTokenAccount,
    #[msg("NoCreatorTokenAccount")]
    NoCreatorTokenAccount,
    #[msg("IncorrectCreatorTokenAccount")]
    IncorrectCreatorTokenAccount,
    #[msg("NumericalOverflowError")]
    NumericalOverflowError,
}
