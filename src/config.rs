use std::collections::HashMap;

use crate::shoulder::Shoulder;

/// The Betanumeric alphabet used for ARK blades.
pub const BETANUMERIC: &[u8] = b"0123456789bcdfghjkmnpqrstvwxz";

/// The application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// The NAAN (Name Assigning Authority Number) for this service.
    pub naan: String,
    /// The default blade length for minted ARKs, excluding the check character.
    /// If a shoulder uses check characters, the final blade will be one character longer.
    /// Used when a shoulder doesn't specify its own blade_length.
    pub default_blade_length: usize,
    /// The maximum number of ARKs that can be minted in a single request.
    pub max_mint_count: usize,
    /// The mapping of shoulders to their configurations.
    pub shoulders: HashMap<String, Shoulder>,
}
