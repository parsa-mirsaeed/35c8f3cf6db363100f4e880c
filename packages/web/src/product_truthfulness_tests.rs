#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn rust_sources(root: &Path, files: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(root).expect("read source directory") {
            let entry = entry.expect("read source entry");
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|name| name.to_str()) == Some("i18n") {
                    continue;
                }
                rust_sources(&path, files);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }

    #[test]
    fn production_web_source_contains_no_known_fictional_or_noop_product_content() {
        let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_sources(&source_root, &mut files);

        let forbidden = [
            "Student dashboard is under development",
            "Parent dashboard is under development",
            "Alex Johnson",
            "Dr. Sarah Johnson",
            "Prof. Michael Chen",
            "Dr. Robert Wilson",
            "Dr. Emily Martinez",
            "Emma Johnson",
            "Michael Johnson",
            "Sophia Johnson",
            "Tuesday, March 18, 2025",
            "March 15, 2025",
            "March 10, 2025",
            "March 5, 2025",
            "onclick: move |_| {}",
            "href: \"#\"",
            "\"99.9%\"",
            "\"120ms\"",
            "\"24/156\"",
            "\"3.7\"",
            "\"+0.3\"",
            "\"87%\"",
        ];

        let mut violations = Vec::new();
        for path in files {
            if path.file_name().and_then(|name| name.to_str())
                == Some("product_truthfulness_tests.rs")
            {
                continue;
            }
            let source = fs::read_to_string(&path).expect("read Rust source");
            for token in forbidden {
                if source.contains(token) {
                    violations.push(format!("{} contains {token:?}", path.display()));
                }
            }
        }

        assert!(
            violations.is_empty(),
            "production truthfulness violations:\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn direct_role_routes_do_not_render_placeholder_dashboards() {
        let main_source = include_str!("main.rs");
        assert!(!main_source.contains("DashboardPlaceholder"));
        assert!(!main_source.contains("under development"));
        for component in [
            "fn PlatformAdminRoute()",
            "fn SchoolManagerRoute()",
            "fn TeacherRoute()",
            "fn StudentRoute()",
            "fn ParentRoute()",
        ] {
            assert!(main_source.contains(component));
        }
    }

    #[test]
    fn canonical_role_dashboards_are_not_shadowed_by_legacy_v2_modules() {
        let school_manager_mod = include_str!("views/role_based/school_manager/mod.rs");
        assert!(school_manager_mod.contains("pub use dashboard::{SchoolManagerDashboard"));
        assert!(!school_manager_mod.contains("dashboard_v2"));

        let teacher_mod = include_str!("views/role_based/teacher/mod.rs");
        assert!(teacher_mod.contains("pub use dashboard::{TeacherDashboard"));
        assert!(!teacher_mod.contains("dashboard_v2"));
    }

    #[test]
    fn incomplete_domains_are_not_advertised_by_production_navigation() {
        let capabilities = api::product_capabilities::PRODUCTION_PRODUCT_CAPABILITIES;
        assert!(!capabilities.attendance);
        assert!(!capabilities.timetable);
        assert!(!capabilities.grade_trends);
        assert!(!capabilities.parent_reports);
        assert!(!capabilities.parent_teacher_communication);
        assert!(!capabilities.school_manager_reports);
        assert!(!capabilities.derived_academic_metrics);
        assert!(!capabilities.synthetic_system_health);
    }

    #[test]
    fn dashboard_shell_does_not_advertise_inert_global_search() {
        let header = include_str!("views/role_based/components/header.rs");
        assert!(!header.contains("placeholder: \"{t_search}...\""));
        assert!(!header.contains("Mobile Search Icon"));
        assert!(!header.contains("common.search"));
    }

    #[test]
    fn mobile_dashboard_navigation_does_not_drop_role_destinations() {
        let layout = include_str!("views/role_based/components/dashboard_layout.rs");
        assert!(!layout.contains(".take(4)"));
        assert!(!layout.contains("window.inner_width"));
        assert!(layout.contains("Sidebar"));
    }

    #[test]
    fn dashboard_remake_stylesheet_is_loaded_after_base_styles() {
        let main_source = include_str!("main.rs");
        let base = main_source
            .find("href: MAIN_CSS")
            .expect("base stylesheet should be linked");
        let remake = main_source
            .find("href: DASHBOARD_REMAKE_CSS")
            .expect("dashboard remake stylesheet should be linked");
        assert!(
            base < remake,
            "dashboard overrides must load after base styles"
        );
    }

    #[test]
    fn provisioning_ui_cannot_call_password_bearing_legacy_endpoint() {
        let source = include_str!("views/role_based/school_manager/user_creation.rs");
        assert!(source.contains("provision_school_user"));
        assert!(!source.contains("CreateUserPayload"));
        assert!(!source.contains("create_user("));
        assert!(!source.contains("Uuid::new_v4"));
        for fake_live_value in [
            "94%",
            "Recent activity",
            "new this week",
            "pending approval",
        ] {
            assert!(!source
                .to_ascii_lowercase()
                .contains(&fake_live_value.to_ascii_lowercase()));
        }
    }

    #[test]
    fn teacher_publish_ui_uses_guided_domain_contract() {
        let source = include_str!("views/role_based/teacher/assignments.rs");
        assert!(source.contains("publish_assignment_guided"));
        assert!(!source.contains("publish_assignment("));
        assert!(source.contains("active enrolled students"));
    }

    #[test]
    fn student_assignment_ui_never_invents_missing_points() {
        let source = include_str!("views/role_based/student/assignments.rs");
        assert!(source.contains("Points not specified"));
        assert!(!source.contains("unwrap_or_else(|| \"100\""));
    }

    #[test]
    fn password_ui_does_not_collect_unverified_current_password() {
        let source = include_str!("views/role_based/school_manager/settings/security.rs");
        assert!(!source.contains("change_admin_password"));
        assert!(!source.contains("current_password"));
    }

    #[test]
    fn general_settings_language_options_match_runtime_locales() {
        let source = include_str!("views/role_based/school_manager/settings/general.rs");
        assert!(source.contains("Locale::all()"));
        for unsupported in ["Español", "Français", "Deutsch", "العربية", "中文"] {
            assert!(!source.contains(unsupported));
        }
    }

    #[test]
    fn notification_settings_do_not_advertise_unavailable_delivery_channels() {
        let source = include_str!("views/role_based/school_manager/settings/notifications.rs");
        assert!(source.contains("email_notifications: Some(false)"));
        assert!(source.contains("push_notifications: Some(false)"));
        assert!(source.contains("notify_report_generated: Some(false)"));
        assert!(source.contains("email_digest_frequency: Some(\"never\".to_string())"));
        assert!(!source.contains("email_notifications.set"));
        assert!(!source.contains("push_notifications.set"));
    }

    #[test]
    fn knowledge_upload_ui_preflights_storage_without_echoing_provider_error_bodies() {
        let source = include_str!("views/role_based/school_manager/knowledge_upload.rs");
        assert!(source.contains("get_knowledge_storage_readiness"));
        assert!(source.contains("Retry storage check"));
        assert!(!source.contains("get(\"error\")"));
        assert!(!source.contains("format!(\"Upload failed: {error}\")"));
    }

    #[test]
    fn parent_ui_uses_parent_user_scoped_contract() {
        let dashboard = include_str!("views/role_based/parent/dashboard.rs");
        let children = include_str!("views/role_based/parent/children.rs");
        assert!(dashboard.contains("get_parent_children_scoped"));
        assert!(children.contains("ParentChildSummary"));
        assert!(!dashboard.contains("get_parent_dashboard_stats"));
        assert!(!children.contains("dashboard_functions::ChildInfo"));
    }
}
