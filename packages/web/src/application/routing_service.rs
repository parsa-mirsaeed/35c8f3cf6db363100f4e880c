use crate::domain::{AccessControl, SystemRole, User};
use dioxus::prelude::*;

/// Application routing service.
pub struct RoutingService;

impl RoutingService {
    /// Get dashboard route for user based on their role.
    pub fn get_role_based_route(user: &User) -> &'static str {
        match user.role {
            SystemRole::PlatformAdmin => "/dashboard/platform-admin",
            SystemRole::SchoolManager => "/dashboard/school-manager",
            SystemRole::Teacher => "/dashboard/teacher",
            SystemRole::Student => "/dashboard/student",
            SystemRole::Parent => "/dashboard/parent",
        }
    }

    /// Check if user can access a specific route.
    pub fn can_access_route(user: &User, route: &str) -> bool {
        if Self::is_public_route(route) {
            return true;
        }

        // Role-specific checks must run before the generic authenticated-route
        // check, otherwise every authenticated user could access every dashboard.
        if let Some(required_role) = Self::get_required_role_for_route(route) {
            return user.role == required_role;
        }

        if let Some(required_permission) = Self::get_required_permission_for_route(route) {
            return user.has_permission(required_permission);
        }

        Self::is_protected_route(route)
    }

    pub fn is_public_route(route: &str) -> bool {
        matches!(
            route,
            "/" | "/login" | "/forgot-password" | "/reset-password"
        )
    }

    pub fn is_protected_route(route: &str) -> bool {
        route == "/dashboard"
            || route.starts_with("/dashboard/overview")
            || route.starts_with("/profile")
            || route.starts_with("/settings")
    }

    pub fn get_required_role_for_route(route: &str) -> Option<SystemRole> {
        match route {
            value if value.starts_with("/dashboard/platform-admin") => {
                Some(SystemRole::PlatformAdmin)
            }
            value if value.starts_with("/dashboard/school-manager") => {
                Some(SystemRole::SchoolManager)
            }
            value if value.starts_with("/dashboard/teacher") => Some(SystemRole::Teacher),
            value if value.starts_with("/dashboard/student") => Some(SystemRole::Student),
            value if value.starts_with("/dashboard/parent") => Some(SystemRole::Parent),
            _ => None,
        }
    }

    pub fn get_required_permission_for_route(route: &str) -> Option<&'static str> {
        match route {
            value if value.contains("/knowledge-assets") => Some("review_knowledge_assets"),
            value if value.contains("/reports") => Some("view_reports"),
            value if value.contains("/classes/manage") => Some("manage_classes"),
            value if value.contains("/assignments/create") => Some("create_assignments"),
            value if value.contains("/assignments/grade") => Some("grade_assignments"),
            value if value.contains("/admin") => Some("manage_users"),
            _ => None,
        }
    }

    pub fn get_navigation_items(
        user: &User,
        locale: &crate::i18n::LocaleContext,
    ) -> Vec<NavigationItem> {
        let mut items = vec![NavigationItem {
            id: "overview".to_string(),
            label: locale.t("nav.overview"),
            icon: "grid_view".to_string(),
            route: "/dashboard/overview".to_string(),
            active: false,
        }];

        match user.role {
            SystemRole::PlatformAdmin => {
                items.extend_from_slice(&[
                    NavigationItem {
                        id: "knowledge-assets".to_string(),
                        label: "Knowledge Assets".to_string(),
                        icon: "library_books".to_string(),
                        route: "/dashboard/platform-admin/knowledge-assets".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "knowledge-audit".to_string(),
                        label: "Knowledge Audit".to_string(),
                        icon: "policy".to_string(),
                        route: "/dashboard/platform-admin/knowledge-audit".to_string(),
                        active: false,
                    },
                ]);
            }
            SystemRole::SchoolManager => {
                items.extend_from_slice(&[
                    NavigationItem {
                        id: "users".to_string(),
                        label: locale.t("nav.user_management"),
                        icon: "groups".to_string(),
                        route: "/dashboard/school-manager/users".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "classes".to_string(),
                        label: locale.t("nav.class_management"),
                        icon: "class".to_string(),
                        route: "/dashboard/school-manager/classes".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "knowledge-submissions".to_string(),
                        label: "Knowledge Submissions".to_string(),
                        icon: "upload_file".to_string(),
                        route: "/dashboard/school-manager/knowledge-submissions".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "reports".to_string(),
                        label: locale.t("nav.reports"),
                        icon: "bar_chart".to_string(),
                        route: "/dashboard/school-manager/reports".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "settings".to_string(),
                        label: locale.t("nav.settings"),
                        icon: "settings".to_string(),
                        route: "/dashboard/school-manager/settings".to_string(),
                        active: false,
                    },
                ]);
            }
            SystemRole::Teacher => {
                items.extend_from_slice(&[
                    NavigationItem {
                        id: "classes".to_string(),
                        label: locale.t("nav.my_classes"),
                        icon: "class".to_string(),
                        route: "/dashboard/teacher/classes".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "assignments".to_string(),
                        label: locale.t("nav.assignments"),
                        icon: "assignment".to_string(),
                        route: "/dashboard/teacher/assignments".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "knowledge-assets".to_string(),
                        label: "Knowledge Assets".to_string(),
                        icon: "library_books".to_string(),
                        route: "/dashboard/teacher/knowledge-assets".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "submissions".to_string(),
                        label: locale.t("nav.grading"),
                        icon: "grading".to_string(),
                        route: "/dashboard/teacher/submissions".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "students".to_string(),
                        label: locale.t("nav.students"),
                        icon: "people".to_string(),
                        route: "/dashboard/teacher/students".to_string(),
                        active: false,
                    },
                ]);
            }
            SystemRole::Student => {
                items.extend_from_slice(&[
                    NavigationItem {
                        id: "classes".to_string(),
                        label: locale.t("nav.my_classes"),
                        icon: "class".to_string(),
                        route: "/dashboard/student/classes".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "assignments".to_string(),
                        label: locale.t("nav.assignments"),
                        icon: "assignment".to_string(),
                        route: "/dashboard/student/assignments".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "grades".to_string(),
                        label: locale.t("nav.grades"),
                        icon: "grade".to_string(),
                        route: "/dashboard/student/grades".to_string(),
                        active: false,
                    },
                ]);
            }
            SystemRole::Parent => {
                items.extend_from_slice(&[
                    NavigationItem {
                        id: "children".to_string(),
                        label: locale.t("nav.children"),
                        icon: "child_care".to_string(),
                        route: "/dashboard/parent/children".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "progress".to_string(),
                        label: locale.t("nav.progress"),
                        icon: "trending_up".to_string(),
                        route: "/dashboard/parent/progress".to_string(),
                        active: false,
                    },
                    NavigationItem {
                        id: "communication".to_string(),
                        label: locale.t("nav.communication"),
                        icon: "chat".to_string(),
                        route: "/dashboard/parent/communication".to_string(),
                        active: false,
                    },
                ]);
            }
        }

        items.push(NavigationItem {
            id: "profile".to_string(),
            label: locale.t("nav.profile"),
            icon: "person_outline".to_string(),
            route: "/profile".to_string(),
            active: false,
        });
        items
    }

    pub fn get_active_navigation_item<'a>(
        navigation_items: &'a [NavigationItem],
        current_route: &str,
    ) -> Option<&'a NavigationItem> {
        navigation_items
            .iter()
            .find(|item| current_route.starts_with(&item.route) || current_route.contains(&item.id))
    }

    pub fn redirect_to_role_dashboard(user: &User) -> String {
        Self::get_role_based_route(user).to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavigationItem {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub route: String,
    pub active: bool,
}

impl NavigationItem {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        icon: impl Into<String>,
        route: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            route: route.into(),
            active: false,
        }
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

/// Route guard components.
pub struct RouteGuard;

impl RouteGuard {
    pub fn require_route_access(
        route: String,
        fallback: Option<Element>,
        children: Element,
    ) -> Element {
        let mut current_user = use_signal(|| None);

        use_effect(move || {
            spawn(async move {
                if let Ok(Some(user)) = crate::application::AppAuthService::get_current_user().await
                {
                    current_user.set(Some(user));
                }
            });
        });

        let access_denied_content = rsx! {
            div {
                style: "padding: 2rem; text-align: center;",
                h2 { "Access Denied" }
                p { "You don't have permission to access this page." }
            }
        };
        let loading_content = rsx! {
            div {
                style: "padding: 2rem; text-align: center;",
                p { "Loading..." }
            }
        };

        rsx! {
            if let Some(user) = current_user.read().as_ref() {
                if RoutingService::can_access_route(user, &route) {
                    {children}
                } else if let Some(fallback_content) = fallback {
                    {fallback_content}
                } else {
                    {access_denied_content}
                }
            } else {
                {loading_content}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_routes_are_not_generic_authenticated_routes() {
        assert!(!RoutingService::is_protected_route(
            "/dashboard/platform-admin/knowledge-assets"
        ));
        assert_eq!(
            RoutingService::get_required_role_for_route(
                "/dashboard/platform-admin/knowledge-assets"
            ),
            Some(SystemRole::PlatformAdmin)
        );
    }
}
