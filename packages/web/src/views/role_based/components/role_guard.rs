use dioxus::prelude::*;
use crate::domain::{User, SystemRole};
use crate::application::{AuthHooks, RoutingService};
use crate::infrastructure::auth_provider::IS_INITIALIZING;
use crate::components::DashboardSkeleton;
use crate::Route;
use web_sys::window;

/// Role guard component that only renders children if user has required role
#[component]
pub fn RoleGuard(
    required_role: SystemRole,
    fallback: Option<Element>,
    children: Element,
) -> Element {
    // Check if auth is still initializing
    let is_initializing = IS_INITIALIZING.read();
    if *is_initializing {
        return rsx! { DashboardSkeleton {} };
    }
    
    // FIX: Call hook directly
    let current_user_val = AuthHooks::use_current_user().ok().flatten();

    if let Some(current_user) = current_user_val {
        if current_user.role == required_role {
            rsx! { {children} }
        } else if let Some(fallback_content) = fallback {
            rsx! { {fallback_content} }
        } else {
            rsx! {
                RoleAccessDeniedMessage {
                    user: current_user,
                    required_role: required_role,
                }
            }
        }
    } else {
        // User not authenticated - Redirect to login
        let nav = use_navigator();
        use_effect(move || {
            nav.push(crate::Route::LoginPage {});
        });
        rsx! { DashboardSkeleton {} }
    }
}

/// Multi-role guard that allows access if user has any of the specified roles
#[component]
pub fn MultiRoleGuard(
    required_roles: Vec<SystemRole>,
    fallback: Option<Element>,
    children: Element,
) -> Element {
    // Check if auth is still initializing
    let is_initializing = IS_INITIALIZING.read();
    if *is_initializing {
        return rsx! { DashboardSkeleton {} };
    }
    
    // FIX: Call hook directly
    let current_user_val = AuthHooks::use_current_user().ok().flatten();

    if let Some(current_user) = current_user_val {
        if required_roles.contains(&current_user.role) {
            rsx! { {children} }
        } else if let Some(fallback_content) = fallback {
            rsx! { {fallback_content} }
        } else {
            rsx! {
                MultiRoleAccessDeniedMessage {
                    user: current_user,
                    required_roles: required_roles,
                }
            }
        }
    } else {
        // Redirect to login
        let nav = use_navigator();
        use_effect(move || {
            nav.push(crate::Route::LoginPage {});
        });
        rsx! { DashboardSkeleton {} }
    }
}

/// Permission guard that only renders children if user has required permission
#[component]
pub fn PermissionGuard(
    required_permission: String,
    fallback: Option<Element>,
    children: Element,
) -> Element {
    // Check if auth is still initializing
    let is_initializing = IS_INITIALIZING.read();
    if *is_initializing {
        return rsx! { LoadingSpinner {} };
    }
    
    // FIX: Call hook directly
    let current_user_val = AuthHooks::use_current_user().ok().flatten();

    if let Some(current_user) = current_user_val {
        if current_user.has_permission(&required_permission) {
            rsx! { {children} }
        } else if let Some(fallback_content) = fallback {
            rsx! { {fallback_content} }
        } else {
            rsx! {
                PermissionDeniedMessage {
                    user: current_user,
                    required_permission: required_permission,
                }
            }
        }
    } else {
        // Redirect to login
        let nav = use_navigator();
        use_effect(move || {
            nav.push(crate::Route::LoginPage {});
        });
        rsx! { DashboardSkeleton {} }
    }
}

/// Authentication guard that only renders children if user is authenticated
#[component]
pub fn AuthGuard(
    fallback: Option<Element>,
    children: Element,
) -> Element {
    // Check if auth is still initializing - show skeleton while checking
    let is_initializing = IS_INITIALIZING.read();
    if *is_initializing {
        return rsx! { DashboardSkeleton {} };
    }
    
    let is_authenticated = AuthHooks::use_is_authenticated();

    if is_authenticated {
        rsx! { {children} }
    } else if let Some(fallback_content) = fallback {
        rsx! { {fallback_content} }
    } else {
        // Redirect to login
        let nav = use_navigator();
        use_effect(move || {
            nav.push(crate::Route::LoginPage {});
        });
        rsx! { DashboardSkeleton {} }
    }
}

/// Route guard component for protecting routes
#[component]
pub fn RouteGuard(
    route: String,
    fallback: Option<Element>,
    children: Element,
) -> Element {
    // Check if auth is still initializing
    let is_initializing = IS_INITIALIZING.read();
    if *is_initializing {
        return rsx! { DashboardSkeleton {} };
    }
    
    // FIX: Call hook directly
    let current_user_val = AuthHooks::use_current_user().ok().flatten();

    if let Some(current_user) = current_user_val {
        if RoutingService::can_access_route(&current_user, &route) {
            rsx! { {children} }
        } else if let Some(fallback_content) = fallback {
            rsx! { {fallback_content} }
        } else {
            rsx! {
                RouteAccessDeniedMessage {
                    user: current_user,
                    route: route,
                }
            }
        }
    } else {
        // Redirect to login
        let nav = use_navigator();
        use_effect(move || {
            nav.push(crate::Route::LoginPage {});
        });
        rsx! { DashboardSkeleton {} }
    }
}

/// Access denied message component
#[component]
pub fn RoleAccessDeniedMessage(
    user: User,
    required_role: SystemRole,
) -> Element {
    rsx! {
        div {
            style: "display: flex; justify-content: center; align-items: center; min-height: 60vh; padding: 2rem;",

            div {
                style: "text-align: center; max-width: 500px; background: white; padding: 3rem; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.1);",

                div {
                    style: "width: 80px; height: 80px; background: #fee2e2; border-radius: 50%; display: flex; align-items: center; justify-content: center; margin: 0 auto 2rem auto;",
                    span {
                        style: "font-size: 2rem;",
                        "🚫"
                    }
                }

                h1 {
                    style: "color: #dc2626; margin-bottom: 1rem; font-size: 1.5rem;",
                    "Access Denied"
                }

                p {
                    style: "color: #6b7280; margin-bottom: 1rem;",
                    "You need to be a {required_role.display_name()} to access this page."
                }

                p {
                    style: "color: #374151; margin-bottom: 2rem;",
                    "Your current role: {user.role.display_name()}"
                }

                div {
                    style: "display: flex; gap: 1rem; justify-content: center;",

                    button {
                        style: "background: #3b82f6; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 6px; cursor: pointer; font-weight: 500;",
                        onclick: move |_| {
                            let nav = use_navigator();
                            let route_str = RoutingService::get_role_based_route(&user);
                            nav.push(route_str);
                        },
                        "Go to Your Dashboard"
                    }

                    button {
                        style: "background: #6b7280; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 6px; cursor: pointer; font-weight: 500;",
                        onclick: move |_| {
                            if let Some(win) = window() {
                                let _ = win.alert_with_message("Please contact the school administrator for access.");
                            }
                        },
                        "Contact Administrator"
                    }
                }
            }
        }
    }
}

/// Access denied message for multiple roles
#[component]
pub fn MultiRoleAccessDeniedMessage(
    user: User,
    required_roles: Vec<SystemRole>,
) -> Element {
    let role_names = required_roles.iter()
        .map(|role| role.display_name())
        .collect::<Vec<_>>()
        .join(" or ");

    rsx! {
        div {
            style: "display: flex; justify-content: center; align-items: center; min-height: 60vh; padding: 2rem;",

            div {
                style: "text-align: center; max-width: 500px; background: white; padding: 3rem; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.1);",

                h1 {
                    style: "color: #dc2626; margin-bottom: 1rem; font-size: 1.5rem;",
                    "Access Denied"
                }

                p {
                    style: "color: #6b7280; margin-bottom: 1rem;",
                    "You need to be one of these roles to access this page: {role_names}"
                }

                p {
                    style: "color: #374151; margin-bottom: 2rem;",
                    "Your current role: {user.role.display_name()}"
                }

                button {
                    style: "background: #3b82f6; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 6px; cursor: pointer; font-weight: 500;",
                    onclick: move |_| {
                        let nav = use_navigator();
                        let route_str = RoutingService::get_role_based_route(&user);
                        nav.push(route_str);
                    },
                    "Go to Your Dashboard"
                }
            }
        }
    }
}

/// Permission denied message component
#[component]
pub fn PermissionDeniedMessage(
    user: User,
    required_permission: String,
) -> Element {
    rsx! {
        div {
            style: "display: flex; justify-content: center; align-items: center; min-height: 60vh; padding: 2rem;",

            div {
                style: "text-align: center; max-width: 500px; background: white; padding: 3rem; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.1);",

                h1 {
                    style: "color: #dc2626; margin-bottom: 1rem; font-size: 1.5rem;",
                    "Permission Denied"
                }

                p {
                    style: "color: #6b7280; margin-bottom: 2rem;",
                    "You don't have the required permission ({required_permission}) to access this page."
                }

                button {
                    style: "background: #3b82f6; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 6px; cursor: pointer; font-weight: 500;",
                    onclick: move |_| {
                        let nav = use_navigator();
                        let route_str = RoutingService::get_role_based_route(&user);
                        nav.push(route_str);
                    },
                    "Go to Dashboard"
                }
            }
        }
    }
}

/// Route access denied message component
#[component]
pub fn RouteAccessDeniedMessage(
    user: User,
    route: String,
) -> Element {
    rsx! {
        div {
            style: "display: flex; justify-content: center; align-items: center; min-height: 60vh; padding: 2rem;",

            div {
                style: "text-align: center; max-width: 500px; background: white; padding: 3rem; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.1);",

                h1 {
                    style: "color: #dc2626; margin-bottom: 1rem; font-size: 1.5rem;",
                    "Route Access Denied"
                }

                p {
                    style: "color: #6b7280; margin-bottom: 2rem;",
                    "You don't have permission to access the route: {route}"
                }

                button {
                    style: "background: #3b82f6; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 6px; cursor: pointer; font-weight: 500;",
                    onclick: move |_| {
                        let nav = use_navigator();
                        let route_str = RoutingService::get_role_based_route(&user);
                        nav.push(route_str);
                    },
                    "Go to Dashboard"
                }
            }
        }
    }
}

/// Not authenticated message component
#[component]
pub fn NotAuthenticatedMessage() -> Element {
    rsx! {
        div {
            style: "display: flex; justify-content: center; align-items: center; min-height: 60vh; padding: 2rem;",
            
            div {
                style: "text-align: center; max-width: 400px; background: white; padding: 3rem; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.1);",
                
                h1 {
                    style: "color: #dc2626; margin-bottom: 1rem; font-size: 1.5rem;",
                    "Authentication Required"
                }
                
                p {
                    style: "color: #6b7280; margin-bottom: 2rem;",
                    "Please log in to access this page."
                }
                
                button {
                    style: "background: #3b82f6; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 6px; cursor: pointer; font-weight: 500;",
                    onclick: move |_| {
                        let nav = use_navigator();
                        nav.push(Route::LoginPage {});
                    },
                    "Go to Login"
                }
            }
        }
    }
}

/// Loading spinner component
#[component]
pub fn LoadingSpinner() -> Element {
    rsx! {
        div {
            style: "text-align: center;",
            div {
                style: "width: 40px; height: 40px; border: 3px solid #e5e7eb; border-top: 3px solid #3b82f6; border-radius: 50%; animation: spin 1s linear infinite; margin: 0 auto 1rem;",
            }
            p {
                style: "color: #6b7280;",
                "Loading..."
            }
        }
    }
}

/// Admin-only guard component (for backward compatibility)
#[component]
pub fn AdminOnly(
    fallback: Option<Element>,
    children: Element,
) -> Element {
    rsx! {
        RoleGuard {
            required_role: SystemRole::SchoolManager,
            fallback,
            children
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SystemRole;

    fn create_test_user(role: SystemRole) -> User {
        User::new(
            "test-1".to_string(),
            "test@example.com".to_string(),
            role,
            None,
        )
    }

    #[test]
    fn test_role_guard_logic() {
        let admin = create_test_user(SystemRole::SchoolManager);
        let teacher = create_test_user(SystemRole::Teacher);

        assert!(admin.role == SystemRole::SchoolManager);
        assert!(teacher.role != SystemRole::SchoolManager);
    }

    #[test]
    fn test_permission_checking() {
        let admin = create_test_user(SystemRole::SchoolManager);
        let teacher = create_test_user(SystemRole::Teacher);
        let student = create_test_user(SystemRole::Student);

        assert!(admin.has_permission("manage_users"));
        assert!(!teacher.has_permission("manage_users"));
        assert!(!student.has_permission("manage_users"));

        assert!(teacher.has_permission("manage_classes"));
        assert!(!student.has_permission("manage_classes"));

        assert!(student.has_permission("submit_assignments"));
        assert!(!teacher.has_permission("submit_assignments"));
    }
}