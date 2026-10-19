/// Single authoritative record of the Soroban protocol version this contract
/// targets. All budget-guardrail test ceilings are derived from this constant.
///
/// # How to perform a protocol bump
///
/// 1. Update `SOROBAN_PROTOCOL_VERSION` to the new protocol number.
/// 2. Add a new match arm in `calibrated_limits` for the new version with
///    the updated ceilings from the Stellar protocol documentation.
/// 3. Run `cargo test -p fluxora_stream` — the build will fail until step 2 is
///    complete, because `LIMITS` fails to evaluate at compile time when the
///    pinned version has no arm.
///
/// # Current pin
///
/// Protocol 21 corresponds to `soroban-sdk 21.7.7` (see `Cargo.toml`).
/// Do not advance past the protocol version live on the target Stellar network.
pub const SOROBAN_PROTOCOL_VERSION: u32 = 21;

/// Resource ceilings derived from a specific Soroban protocol version.
///
/// All fields represent **maximum** allowed values for the named operation.
/// Tests assert that measured costs stay at or below these ceilings.
#[derive(Debug)]
pub struct ProtocolBudgetLimits {
    /// Maximum CPU instructions for a single `withdraw` call (hot path).
    pub single_withdraw_cpu_max: u64,
    /// Maximum memory bytes for a single `withdraw` call (hot path).
    pub single_withdraw_mem_max: u64,
    /// Maximum CPU instructions for `batch_withdraw` over 10 streams.
    pub batch_withdraw_10_cpu_max: u64,
    /// Maximum memory bytes for `batch_withdraw` over 10 streams.
    pub batch_withdraw_10_mem_max: u64,
    /// Maximum CPU instructions for `create_streams` with 5 entries.
    pub create_streams_5_cpu_max: u64,
    /// Maximum memory bytes for `create_streams` with 5 entries.
    pub create_streams_5_mem_max: u64,
}

/// Looks up the resource ceilings for a Soroban protocol version.
///
/// This is the single source of truth for the calibrated versions. It is
/// `const`, so it can be folded into [`LIMITS`] at compile time, and it
/// returns [`None`] for uncalibrated versions instead of panicking. The two
/// public entry points ([`LIMITS`] and [`limits_for`]) decide how an
/// uncalibrated version is reported: a build error at the pin, or a named
/// panic at runtime.
pub const fn calibrated_limits(version: u32) -> Option<ProtocolBudgetLimits> {
    match version {
        21 => Some(ProtocolBudgetLimits {
            // Ceilings measured against soroban-sdk 21.7.7 on protocol 21.
            // Source: empirical guardrail tests in contracts/stream/src/test.rs.
            // Update this block when migrating to a new protocol version.
            single_withdraw_cpu_max: 1_000_000,
            single_withdraw_mem_max: 500_000,
            batch_withdraw_10_cpu_max: 5_000_000,
            batch_withdraw_10_mem_max: 2_000_000,
            create_streams_5_cpu_max: 3_000_000,
            create_streams_5_mem_max: 1_500_000,
        }),
        _ => None,
    }
}

/// Maps a Soroban protocol version number to its resource ceilings, naming the
/// requested version when it has no arm.
///
/// Use this from code that needs the runtime panic message. [`LIMITS`] is the
/// compile-time equivalent for the pinned version.
///
/// # Panics
///
/// Panics with a message naming `version` when it is not a calibrated Soroban
/// protocol number.
pub fn limits_for(version: u32) -> ProtocolBudgetLimits {
    match calibrated_limits(version) {
        Some(limits) => limits,
        None => panic!(
            "unsupported Soroban protocol version {version} — add a match arm in protocol_limits.rs"
        ),
    }
}

/// Pre-computed resource ceilings for the pinned protocol version.
///
/// Import this constant in budget-guardrail tests:
///
/// ```rust
/// use crate::protocol_limits::LIMITS;
///
/// assert!(cpu <= LIMITS.single_withdraw_cpu_max, "...");
/// ```
///
/// Changing `SOROBAN_PROTOCOL_VERSION` to an unrecognised value causes this
/// constant to fail at compile time, so CI cannot pass with a stale pin.
pub const LIMITS: ProtocolBudgetLimits = match calibrated_limits(SOROBAN_PROTOCOL_VERSION) {
    Some(limits) => limits,
    None => panic!("SOROBAN_PROTOCOL_VERSION has no calibrated limits — add a match arm in protocol_limits.rs"),
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::{String, ToString};

    /// The pinned version must always have a calibrated arm. If this fails, a
    /// bump of `SOROBAN_PROTOCOL_VERSION` reached a version with no ceilings.
    #[test]
    fn pinned_version_is_calibrated() {
        assert!(
            calibrated_limits(SOROBAN_PROTOCOL_VERSION).is_some(),
            "SOROBAN_PROTOCOL_VERSION={SOROBAN_PROTOCOL_VERSION} has no calibrated arm — \
             add one in protocol_limits.rs"
        );
    }

    /// An uncalibrated version must fail with a message naming the version
    /// that was requested, not an anonymous panic.
    #[test]
    fn unsupported_version_panics_naming_the_version() {
        let unsupported = SOROBAN_PROTOCOL_VERSION + 1;
        assert!(
            calibrated_limits(unsupported).is_none(),
            "test precondition: protocol {unsupported} must have no arm"
        );

        let panic = std::panic::catch_unwind(|| limits_for(unsupported))
            .expect_err("limits_for must panic for an uncalibrated version");

        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .expect("panic payload must be a string");

        let expected = unsupported.to_string();
        assert!(
            message.contains(&expected),
            "panic message {message:?} must name the requested version {expected}"
        );
    }

    /// Protocol 21 keeps the ceilings that shipped with it — a drift test to
    /// catch accidental recalibration.
    #[test]
    fn protocol_21_ceilings_are_unchanged() {
        let limits = calibrated_limits(21).expect("protocol 21 must be calibrated");
        assert_eq!(limits.single_withdraw_cpu_max, 1_000_000);
        assert_eq!(limits.single_withdraw_mem_max, 500_000);
        assert_eq!(limits.batch_withdraw_10_cpu_max, 5_000_000);
        assert_eq!(limits.batch_withdraw_10_mem_max, 2_000_000);
        assert_eq!(limits.create_streams_5_cpu_max, 3_000_000);
        assert_eq!(limits.create_streams_5_mem_max, 1_500_000);
    }

    /// `LIMITS` must equal the ceilings of the pinned version, so the constant
    /// and the match arms cannot drift apart.
    #[test]
    fn limits_matches_pinned_version_arms() {
        let pinned = calibrated_limits(SOROBAN_PROTOCOL_VERSION)
            .expect("pinned version must be calibrated");
        assert_eq!(LIMITS.single_withdraw_cpu_max, pinned.single_withdraw_cpu_max);
        assert_eq!(LIMITS.single_withdraw_mem_max, pinned.single_withdraw_mem_max);
        assert_eq!(LIMITS.batch_withdraw_10_cpu_max, pinned.batch_withdraw_10_cpu_max);
        assert_eq!(LIMITS.batch_withdraw_10_mem_max, pinned.batch_withdraw_10_mem_max);
        assert_eq!(LIMITS.create_streams_5_cpu_max, pinned.create_streams_5_cpu_max);
        assert_eq!(LIMITS.create_streams_5_mem_max, pinned.create_streams_5_mem_max);
    }
}
