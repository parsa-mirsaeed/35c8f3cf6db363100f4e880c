use serde::{Deserialize, Serialize};

/// Release-controlled product capabilities.
///
/// These flags are product-truthfulness metadata, not authorization controls.
/// Backend authorization remains authoritative for every endpoint. A capability
/// set to `false` means the production UI must not advertise or simulate the
/// feature until its owning implementation PR supplies real end-to-end behavior.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductCapabilities {
    pub schema_version: u32,
    pub attendance: bool,
    pub timetable: bool,
    pub grade_trends: bool,
    pub parent_reports: bool,
    pub parent_teacher_communication: bool,
    pub school_manager_reports: bool,
    pub synthetic_system_health: bool,
}

impl ProductCapabilities {
    pub const fn production() -> Self {
        Self {
            schema_version: 1,
            attendance: false,
            timetable: false,
            grade_trends: false,
            parent_reports: false,
            parent_teacher_communication: false,
            school_manager_reports: false,
            synthetic_system_health: false,
        }
    }
}

pub const PRODUCTION_PRODUCT_CAPABILITIES: ProductCapabilities = ProductCapabilities::production();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_metadata_matches_the_compiled_production_capabilities() {
        let metadata: ProductCapabilities = serde_json::from_str(include_str!(
            "../../../docs/release/product-capabilities.json"
        ))
        .expect("release capability metadata must be valid JSON");
        assert_eq!(metadata, PRODUCTION_PRODUCT_CAPABILITIES);
    }

    #[test]
    fn incomplete_product_domains_fail_closed_in_the_production_release() {
        let capabilities = PRODUCTION_PRODUCT_CAPABILITIES;
        assert!(!capabilities.attendance);
        assert!(!capabilities.timetable);
        assert!(!capabilities.grade_trends);
        assert!(!capabilities.parent_reports);
        assert!(!capabilities.parent_teacher_communication);
        assert!(!capabilities.school_manager_reports);
        assert!(!capabilities.synthetic_system_health);
    }

    #[test]
    fn placeholder_server_endpoints_are_disabled_by_the_authorization_manifest() {
        let manifest = include_str!("../endpoint_authorization_manifest.psv");
        for endpoint in [
            "dashboard/student/stats",
            "dashboard/student/classes",
            "dashboard/teacher/classes",
            "dashboard/parent/stats",
            "parent/child/attendance",
        ] {
            let expected = format!("server|{endpoint}|Disabled|");
            assert!(
                manifest.lines().any(|line| line.starts_with(&expected)),
                "{endpoint} must remain disabled until it returns non-placeholder production data"
            );
        }
    }
}
