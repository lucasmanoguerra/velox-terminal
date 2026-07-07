//! Order state machine with explicit transitions.

use velox_core::OrderState;
use crate::error::OmsError;

/// Validates state transitions for an order.
/// Returns Err if the transition is invalid.
pub fn validate_transition(from: OrderState, to: OrderState) -> Result<(), OmsError> {
    use OrderState::*;

    let valid = match (from, to) {
        (PendingNew, New) => true,
        (PendingNew, Rejected) => true,
        (New, PartiallyFilled) => true,
        (New, Filled) => true,
        (New, Canceled) => true,
        (New, PendingCancel) => true,
        (New, Expired) => true,
        (PartiallyFilled, PartiallyFilled) => true,
        (PartiallyFilled, Filled) => true,
        (PartiallyFilled, PendingCancel) => true,
        (PartiallyFilled, Canceled) => true,
        (PendingCancel, Canceled) => true,
        (PendingReplace, New) => true, // replace accepted
        (PendingReplace, Rejected) => true, // replace rejected
        _ => false,
    };

    if valid {
        Ok(())
    } else {
        Err(OmsError::InvalidTransition {
            from: format!("{:?}", from),
            to: format!("{:?}", to),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velox_core::OrderState::*;

    #[test]
    fn test_valid_transitions() {
        assert!(validate_transition(PendingNew, New).is_ok());
        assert!(validate_transition(PendingNew, Rejected).is_ok());
        assert!(validate_transition(New, Filled).is_ok());
        assert!(validate_transition(PartiallyFilled, Filled).is_ok());
        assert!(validate_transition(PendingCancel, Canceled).is_ok());
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(validate_transition(New, PendingNew).is_err()); // can't go back
        assert!(validate_transition(Filled, New).is_err());    // can't unfill
        assert!(validate_transition(Canceled, Filled).is_err()); // can't uncancel
        assert!(validate_transition(Rejected, PendingNew).is_err());
    }

    #[test]
    fn test_partial_fill_loop() {
        assert!(validate_transition(PartiallyFilled, PartiallyFilled).is_ok());
    }
}
