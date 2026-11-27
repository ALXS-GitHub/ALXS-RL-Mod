//! Windows application-control checks, read-only: why Windows may refuse to
//! start a third-party program (BakkesMod is unsigned), so the UI can say so
//! instead of failing silently. Nothing here changes a setting.

use serde::Serialize;

/// Smart App Control ("Contrôle intelligent des applications", Windows 11).
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SmartAppControl {
    Off,
    /// Blocks unsigned programs without a known reputation.
    On,
    /// Watching for a while before switching itself on or off.
    Evaluation,
    /// Not reported by this Windows (older builds, Windows 10).
    Unknown,
}

/// `VerifiedAndReputablePolicyState`: 0 off, 1 on, 2 evaluation.
fn from_policy_state(value: Option<u32>) -> SmartAppControl {
    match value {
        Some(0) => SmartAppControl::Off,
        Some(1) => SmartAppControl::On,
        Some(2) => SmartAppControl::Evaluation,
        _ => SmartAppControl::Unknown,
    }
}

#[cfg(windows)]
pub fn smart_app_control() -> SmartAppControl {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let value = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SYSTEM\CurrentControlSet\Control\CI\Policy")
        .and_then(|k| k.get_value::<u32, _>("VerifiedAndReputablePolicyState"))
        .ok();
    from_policy_state(value)
}

#[cfg(not(windows))]
pub fn smart_app_control() -> SmartAppControl {
    SmartAppControl::Unknown
}

/// Win32 errors returned when an application-control policy (Smart App
/// Control, WDAC, AppLocker, SmartScreen reputation, antivirus) refuses to
/// start a program.
const ERROR_VIRUS_INFECTED: i32 = 225;
const ERROR_ACCESS_DISABLED_BY_POLICY: i32 = 1260;
/// `ERROR_SYSTEM_INTEGRITY_*` (policy violation, reputation checks…).
const SYSTEM_INTEGRITY: std::ops::RangeInclusive<i32> = 4550..=4590;

/// The start was refused by Windows' application control, not a missing
/// file or a crash.
pub fn blocked_by_policy(err: &std::io::Error) -> bool {
    err.raw_os_error().is_some_and(is_policy_code)
}

fn is_policy_code(code: i32) -> bool {
    code == ERROR_VIRUS_INFECTED
        || code == ERROR_ACCESS_DISABLED_BY_POLICY
        || SYSTEM_INTEGRITY.contains(&code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_state_values() {
        assert_eq!(from_policy_state(Some(0)), SmartAppControl::Off);
        assert_eq!(from_policy_state(Some(1)), SmartAppControl::On);
        assert_eq!(from_policy_state(Some(2)), SmartAppControl::Evaluation);
        assert_eq!(from_policy_state(None), SmartAppControl::Unknown);
    }

    #[test]
    fn policy_errors() {
        // ERROR_SYSTEM_INTEGRITY_POLICY_VIOLATION (Smart App Control).
        assert!(blocked_by_policy(&std::io::Error::from_raw_os_error(4551)));
        assert!(blocked_by_policy(&std::io::Error::from_raw_os_error(1260)));
        // File not found, elevation required: not a policy block.
        assert!(!blocked_by_policy(&std::io::Error::from_raw_os_error(2)));
        assert!(!blocked_by_policy(&std::io::Error::from_raw_os_error(740)));
    }
}
