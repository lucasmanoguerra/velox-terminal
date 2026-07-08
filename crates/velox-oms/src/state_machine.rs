//! Order state machine with explicit transitions.

use crate::error::OmsError;
use velox_core::OrderState;

/// Validates state transitions for an order.
/// Returns Err if the transition is invalid.
pub fn validate_transition(from: OrderState, to: OrderState) -> Result<(), OmsError> {
    use OrderState::*;

    let valid = match (from, to) {
        // PendingNew transitions
        (PendingNew, New) => true,
        (PendingNew, Rejected) => true,
        (PendingNew, PendingCancel) => true,
        (PendingNew, Expired) => true,

        // New order transitions
        (New, PartiallyFilled) => true,
        (New, Filled) => true,
        (New, Canceled) => true,
        (New, PendingCancel) => true,
        (New, Expired) => true,
        (New, Stopped) => true,
        (New, PendingReplace) => true,

        // PartiallyFilled transitions
        (PartiallyFilled, PartiallyFilled) => true,
        (PartiallyFilled, Filled) => true,
        (PartiallyFilled, PendingCancel) => true,
        (PartiallyFilled, Canceled) => true,
        (PartiallyFilled, Stopped) => true,

        // PendingCancel transitions
        (PendingCancel, Canceled) => true,
        (PendingCancel, New) => true, // cancel rejected by broker

        // PendingReplace transitions
        (PendingReplace, New) => true,           // replace accepted
        (PendingReplace, Rejected) => true,      // replace rejected
        (PendingReplace, PendingCancel) => true, // cancel while replacing

        // Stopped transitions
        (Stopped, New) => true,
        (Stopped, PendingCancel) => true,
        (Stopped, Expired) => true,

        // Terminal states: no transitions out
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

/// All valid transitions as a list of (from, to) pairs.
pub fn all_valid_transitions() -> Vec<(OrderState, OrderState)> {
    use OrderState::*;
    vec![
        (PendingNew, New),
        (PendingNew, Rejected),
        (PendingNew, PendingCancel),
        (PendingNew, Expired),
        (New, PartiallyFilled),
        (New, Filled),
        (New, Canceled),
        (New, PendingCancel),
        (New, Expired),
        (New, Stopped),
        (New, PendingReplace),
        (PartiallyFilled, PartiallyFilled),
        (PartiallyFilled, Filled),
        (PartiallyFilled, PendingCancel),
        (PartiallyFilled, Canceled),
        (PartiallyFilled, Stopped),
        (PendingCancel, Canceled),
        (PendingCancel, New),
        (PendingReplace, New),
        (PendingReplace, Rejected),
        (PendingReplace, PendingCancel),
        (Stopped, New),
        (Stopped, PendingCancel),
        (Stopped, Expired),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use velox_core::OrderState::*;

    // --- Unit tests ---

    #[test]
    fn test_valid_transitions_basic() {
        assert!(validate_transition(PendingNew, New).is_ok());
        assert!(validate_transition(PendingNew, Rejected).is_ok());
        assert!(validate_transition(New, Filled).is_ok());
        assert!(validate_transition(PartiallyFilled, Filled).is_ok());
        assert!(validate_transition(PendingCancel, Canceled).is_ok());
    }

    #[test]
    fn test_invalid_transitions_basic() {
        assert!(validate_transition(New, PendingNew).is_err());
        assert!(validate_transition(Filled, New).is_err());
        assert!(validate_transition(Canceled, Filled).is_err());
        assert!(validate_transition(Rejected, PendingNew).is_err());
    }

    #[test]
    fn test_partial_fill_loop() {
        assert!(validate_transition(PartiallyFilled, PartiallyFilled).is_ok());
    }

    #[test]
    fn test_terminal_states_have_no_outgoing() {
        let terminal = vec![Filled, Canceled, Rejected, Expired];
        let all_states = vec![
            PendingNew,
            New,
            PartiallyFilled,
            Filled,
            Canceled,
            Rejected,
            Expired,
            PendingCancel,
            PendingReplace,
            Stopped,
        ];

        for terminal_state in &terminal {
            for target in &all_states {
                if terminal_state == target {
                    continue;
                }
                assert!(
                    validate_transition(*terminal_state, *target).is_err(),
                    "Terminal state {:?} should not transition to {:?}",
                    terminal_state,
                    target
                );
            }
        }
    }

    #[test]
    fn test_new_order_transitions() {
        // New can go to any of these
        let valid_targets = vec![
            PartiallyFilled,
            Filled,
            Canceled,
            PendingCancel,
            Expired,
            Stopped,
            PendingReplace,
        ];
        for target in &valid_targets {
            assert!(
                validate_transition(New, *target).is_ok(),
                "New -> {:?} should be valid",
                target
            );
        }
    }

    // --- Property-based tests ---

    #[expect(dead_code)]
    fn arb_order_state() -> impl Strategy<Value = OrderState> {
        prop::sample::select(vec![
            PendingNew,
            New,
            PartiallyFilled,
            Filled,
            Canceled,
            Rejected,
            Expired,
            PendingCancel,
            PendingReplace,
            Stopped,
        ])
    }

    proptest! {
        fn property_valid_transitions(from in arb_order_state(), to in arb_order_state()) {
            let valid_pairs = all_valid_transitions();
            let is_valid = valid_pairs.iter().any(|(f, t)| *f == from && *t == to);
            let result = validate_transition(from, to);
            if is_valid {
                prop_assert!(result.is_ok(),
                    "Expected ({:?} -> {:?}) to be valid", from, to);
            }
        }

        fn property_terminal_no_exits(
            terminal in prop::sample::select(vec![Filled, Canceled, Rejected, Expired]),
            target in arb_order_state(),
        ) {
            if terminal == target { return Ok(()); }
            let result = validate_transition(terminal, target);
            prop_assert!(result.is_err(),
                "Terminal {:?} should reject transition to {:?}", terminal, target);
        }

        fn property_no_back_to_pending(from in arb_order_state()) {
            if from == PendingNew { return Ok(()); }
            let result = validate_transition(from, PendingNew);
            prop_assert!(result.is_err(),
                "{:?} should not transition back to PendingNew", from);
        }
    }
}
