use crate::ark::parse_ark;
use crate::config::AppState;
use crate::error::AppError;

/// Resolves a received ARK to the target URL of its shoulder
///
/// The normalized NAAN and shoulder select the shoulder; the target gets the ARK as
/// received, with the label as "ark:".
pub fn resolve(state: &AppState, received: &str) -> Result<String, AppError> {
    let ark = parse_ark(received).map_err(|_| AppError::InvalidArk)?;
    if ark.violation().is_some() {
        return Err(AppError::InvalidArk);
    }
    if ark.naan() != state.naan {
        return Err(AppError::InvalidNaan);
    }
    let shoulder = ark
        .shoulder()
        .and_then(|s| state.shoulders.get(s))
        .ok_or(AppError::ShoulderNotFound)?;

    let target = shoulder.resolve(&ark)?;
    tracing::info!(ark = ?ark.received(), target = %target, "ARK resolved");
    Ok(target)
}
