//! Translation system for EduTalent
//!
//! Provides type-safe translations with compile-time key checking
//! and runtime JSON loading for English and Farsi.

use super::Locale;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Translation key type for type safety
pub type TranslationKey = &'static str;

/// Translations storage - maps locale -> namespace.key -> translated string
static TRANSLATIONS: LazyLock<HashMap<Locale, HashMap<&'static str, &'static str>>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        // English translations
        map.insert(Locale::En, create_en_translations());

        // Farsi translations
        map.insert(Locale::Fa, create_fa_translations());

        map
    });

/// Get a translation for the given key and locale
pub fn translate(key: TranslationKey, locale: Locale) -> String {
    TRANSLATIONS
        .get(&locale)
        .and_then(|translations| translations.get(key))
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            // Fallback: try English, then return key itself
            if locale != Locale::En {
                TRANSLATIONS
                    .get(&Locale::En)
                    .and_then(|t| t.get(key))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| key.to_string())
            } else {
                key.to_string()
            }
        })
}

/// Shorthand function for translation
pub fn t(key: TranslationKey, locale: Locale) -> String {
    translate(key, locale)
}

/// Create English translations
fn create_en_translations() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();

    // ==================== COMMON ====================
    m.insert("common.loading", "Loading...");
    m.insert("common.save", "Save");
    m.insert("common.cancel", "Cancel");
    m.insert("common.delete", "Delete");
    m.insert("common.edit", "Edit");
    m.insert("common.create", "Create");
    m.insert("common.search", "Search");
    m.insert("common.filter", "Filter");
    m.insert("common.view", "View");
    m.insert("common.close", "Close");
    m.insert("common.submit", "Submit");
    m.insert("common.back", "Back");
    m.insert("common.next", "Next");
    m.insert("common.confirm", "Confirm");
    m.insert("common.yes", "Yes");
    m.insert("common.no", "No");
    m.insert("common.success", "Success");
    m.insert("common.error", "Error");
    m.insert("common.warning", "Warning");
    m.insert("common.info", "Info");
    m.insert("common.required", "Required");
    m.insert("common.optional", "Optional");
    m.insert("common.actions", "Actions");
    m.insert("common.status", "Status");
    m.insert("common.date", "Date");
    m.insert("common.time", "Time");
    m.insert("common.time.ago", "ago");
    m.insert("common.time.minute", "min");
    m.insert("common.time.hour", "hour");
    m.insert("common.name", "Name");
    m.insert("common.email", "Email");
    m.insert("common.phone", "Phone");
    m.insert("common.address", "Address");
    m.insert("common.description", "Description");
    m.insert("common.details", "Details");
    m.insert("common.settings", "Settings");
    m.insert("common.profile", "Profile");
    m.insert("common.logout", "Logout");
    m.insert("common.language", "Language");
    m.insert("common.select_language", "Select Language");
    m.insert("common.no_data", "No data available");
    m.insert("common.view_details", "View Details");
    m.insert("common.view_all", "View All");
    m.insert("common.portal", "Portal");
    m.insert("common.open", "Open");
    m.insert("common.subject", "Subject");
    m.insert("common.term", "Term");
    m.insert("common.teacher", "Teacher");
    m.insert("common.students", "Students");
    m.insert("common.list", "List");
    m.insert("common.grid", "Grid");
    m.insert("common.select_subject", "Select a subject...");
    m.insert("common.loading_subjects", "Loading subjects...");
    m.insert("common.retry", "Retry");
    m.insert("common.previous", "Previous");

    // ==================== TIME FORMATTING ====================
    m.insert("time.just_now", "Just now");
    m.insert("time.minutes_ago", "{0}m ago");
    m.insert("time.hours_ago", "{0}h ago");
    m.insert("time.days_ago", "{0}d ago");

    // ==================== AUTH ====================
    m.insert("auth.sign_in", "Sign In");
    m.insert("auth.sign_out", "Sign Out");
    m.insert("auth.sign_up", "Sign Up");
    m.insert("auth.email", "Email Address");
    m.insert("auth.password", "Password");
    m.insert("auth.confirm_password", "Confirm Password");
    m.insert("auth.forgot_password", "Forgot Password?");
    m.insert("auth.reset_password", "Reset Password");
    m.insert("auth.remember_me", "Remember me");
    m.insert("auth.no_account", "Don't have an account?");
    m.insert("auth.have_account", "Already have an account?");
    m.insert("auth.signing_in", "Signing in...");
    m.insert("auth.invalid_credentials", "Invalid email or password");
    m.insert("auth.account_inactive", "Your account has been deactivated");
    m.insert("auth.account_locked", "Your account has been locked");
    m.insert(
        "auth.email_not_confirmed",
        "Please confirm your email address",
    );
    m.insert("auth.invalid_email", "Please enter a valid email address");
    m.insert(
        "auth.password_too_short",
        "Password must be at least 8 characters",
    );
    m.insert(
        "auth.session_expired",
        "Your session has expired, please login again",
    );
    m.insert(
        "auth.unauthorized",
        "You don't have permission to access this resource",
    );
    m.insert("auth.welcome_back", "Welcome Back");
    m.insert(
        "auth.login_subtitle",
        "Enter your credentials to access your account",
    );
    m.insert("auth.protected_by", "Protected by EduTalent Security");

    // ==================== NAVIGATION ====================
    m.insert("nav.dashboard", "Dashboard");
    m.insert("nav.classes", "Classes");
    m.insert("nav.students", "Students");
    m.insert("nav.teachers", "Teachers");
    m.insert("nav.parents", "Parents");
    m.insert("nav.assignments", "Assignments");
    m.insert("nav.grades", "Grades");
    m.insert("nav.schedule", "Schedule");
    m.insert("nav.reports", "Reports");
    m.insert("nav.messages", "Messages");
    m.insert("nav.notifications", "Notifications");
    m.insert("nav.settings", "Settings");
    m.insert("nav.help", "Help");
    m.insert("nav.materials", "Materials");
    m.insert("nav.submissions", "Submissions");
    m.insert("nav.attendance", "Attendance");
    m.insert("nav.users", "Users");
    m.insert("nav.requests", "Requests");
    m.insert("nav.children", "My Children");
    m.insert("nav.communication", "Communication");
    m.insert("nav.overview", "Overview");
    m.insert("nav.user_management", "User Management");
    m.insert("nav.class_management", "Class Management");
    m.insert("nav.my_classes", "My Classes");
    m.insert("nav.grading", "Grading");
    m.insert("nav.progress", "Progress");
    m.insert("nav.profile", "Profile");

    // ==================== MATERIALS ====================
    m.insert("materials.loading", "Loading materials...");
    m.insert("materials.no_materials_title", "No Materials Yet");
    m.insert(
        "materials.no_materials_desc",
        "Your teacher hasn't uploaded any materials for this class yet.",
    );
    m.insert("materials.added_prefix", "Added: ");
    m.insert("materials.failed_load", "Failed to load materials");
    m.insert("materials.add_new", "Add New Material");
    m.insert("materials.title", "Title");
    m.insert("materials.title_placeholder", "e.g., Chapter 5 Notes");
    m.insert("materials.title_required", "Title is required");
    m.insert("materials.description", "Description");
    m.insert(
        "materials.description_placeholder",
        "Briefly describe this material...",
    );
    m.insert("materials.file_url", "File URL");
    m.insert("materials.upload_file", "Upload File");
    m.insert("materials.select_file", "Select a file");
    m.insert("materials.uploading", "Uploading...");
    m.insert("materials.upload_success", "File uploaded successfully");
    m.insert("materials.upload_failed", "Failed to upload file");
    m.insert("materials.or_upload_file", "Or Upload File");
    m.insert("materials.click_to_upload", "Click to select a file");
    m.insert("materials.upload_hint", "PDF, TXT, MD, HTML supported");

    // ==================== PARENT DASHBOARD ====================
    m.insert("parent.dashboard.sections.overview", "Family Overview");
    m.insert(
        "parent.dashboard.sections.children_progress",
        "Children's Progress",
    );
    m.insert("parent.dashboard.sections.quick_actions", "Quick Actions");
    m.insert("parent.dashboard.sections.coming_soon", "Coming Soon");

    m.insert("parent.dashboard.stats.children", "Children");
    m.insert("parent.dashboard.stats.avg_gpa", "Average GPA");
    m.insert("parent.dashboard.stats.messages", "Messages");
    m.insert("parent.dashboard.stats.events", "Events");
    m.insert("parent.dashboard.stats.status.enrolled", "Enrolled");
    m.insert("parent.dashboard.stats.status.family_avg", "Family Avg");
    m.insert("parent.dashboard.stats.status.unread", "Unread");
    m.insert("parent.dashboard.stats.status.upcoming", "Upcoming");

    m.insert(
        "parent.dashboard.empty.no_children",
        "No children linked to your account",
    );
    m.insert(
        "parent.dashboard.empty.contact_admin",
        "Contact school administration to link your children.",
    );

    m.insert("parent.dashboard.actions.view_reports", "View Reports");
    m.insert(
        "parent.dashboard.actions.view_reports_desc",
        "See your children's academic reports.",
    );
    m.insert("parent.dashboard.actions.view_classes", "View Classes");
    m.insert(
        "parent.dashboard.actions.view_classes_desc",
        "See enrolled classes and schedules.",
    );
    m.insert(
        "parent.dashboard.actions.contact_teacher",
        "Contact Teacher",
    );
    m.insert(
        "parent.dashboard.actions.contact_teacher_desc",
        "Send a message to your child's teacher.",
    );

    m.insert("parent.dashboard.coming_soon.chat", "Parent-Teacher Chat");
    m.insert(
        "parent.dashboard.coming_soon.chat_desc",
        "Direct messaging with teachers",
    );
    m.insert("parent.dashboard.coming_soon.calendar", "School Calendar");
    m.insert(
        "parent.dashboard.coming_soon.calendar_desc",
        "View upcoming school events",
    );
    m.insert(
        "parent.dashboard.coming_soon.notifications",
        "Push Notifications",
    );
    m.insert(
        "parent.dashboard.coming_soon.notifications_desc",
        "Get alerts for important updates",
    );

    m.insert("parent.dashboard.child_card.gpa", "GPA");
    m.insert("parent.dashboard.child_card.classes", "Classes");
    m.insert("parent.dashboard.child_card.view_profile", "View Profile");
    m.insert("parent.dashboard.common.coming_soon_badge", "Coming Soon");

    // ==================== DASHBOARD ====================
    m.insert("dashboard.welcome", "Welcome");
    m.insert("dashboard.overview", "Overview");
    m.insert("dashboard.quick_actions", "Quick Actions");
    m.insert("dashboard.recent_activity", "Recent Activity");
    m.insert("dashboard.upcoming", "Upcoming");
    m.insert("dashboard.statistics", "Statistics");
    m.insert("dashboard.total_students", "Total Students");
    m.insert("dashboard.total_teachers", "Total Teachers");
    m.insert("dashboard.total_classes", "Total Classes");
    m.insert("dashboard.pending_grading", "Pending Grading");
    m.insert("dashboard.active_assignments", "Active Assignments");
    m.insert("dashboard.pending_submissions", "Pending Submissions");
    m.insert("dashboard.today_schedule", "Today's Schedule");
    m.insert("dashboard.my_progress", "My Progress");
    m.insert("dashboard.enrolled_classes", "Enrolled Classes");
    m.insert("dashboard.pending_tasks", "Pending Tasks");
    m.insert("dashboard.current_gpa", "Current GPA");
    m.insert("dashboard.attendance", "Attendance");
    m.insert("dashboard.upcoming_assignments", "Upcoming Assignments");
    m.insert("dashboard.my_courses", "My Courses");

    // ==================== GRADES ====================
    m.insert("grades.title", "Grades");
    m.insert(
        "grades.description",
        "Check your grades and academic progress",
    );
    m.insert("grades.gpa", "GPA");
    m.insert("grades.cumulative_gpa", "Cumulative GPA");
    m.insert("grades.current_gpa", "Current GPA");
    m.insert("grades.credits_completed", "Credits Completed");
    m.insert("grades.attendance_rate", "Attendance Rate");
    m.insert("grades.by_class", "Grades by Class");
    m.insert("grades.grade_trends", "Grade Trends");
    m.insert("grades.view_trends", "View Detailed Trends");
    m.insert("grades.performance_analysis", "Performance Analysis");
    m.insert(
        "grades.track_progress",
        "Track your academic progress over time with detailed analytics",
    );
    m.insert("grades.grade_details", "Grade Details");
    m.insert("grades.no_classes", "No enrolled classes found");
    m.insert("grades.no_grades", "No graded assignments yet");
    m.insert("grades.loading", "Loading grades...");
    m.insert("grades.failed_load", "Failed to load");
    m.insert("grades.graded_at", "Graded");
    m.insert("grades.points", "Points");
    m.insert("grades.academic_trends", "Academic Trends Analysis");
    m.insert("grades.gpa_change", "GPA Change This Term");
    m.insert("grades.avg_score", "Avg Assignment Score");
    m.insert("grades.consistent_improvement", "Consistent Improvement");
    m.insert(
        "grades.improvement_desc",
        "Your grades have improved over the last 3 months",
    );
    m.insert("grades.on_time_submissions", "On-time Submissions");
    m.insert("grades.on_time_desc", "of assignments submitted on time");
    m.insert("grades.strong_subject", "Strong Subject");
    m.insert("grades.strong_subject_desc", "Best performance in");
    m.insert(
        "grades.coming_soon",
        "Detailed charts and personalized insights coming soon!",
    );
    m.insert("grades.current_performance", "Current Academic Performance");
    m.insert("grades.scale_100", "out of 100");
    m.insert("grades.scale_20", "out of 20");
    m.insert("grades.total_graded", "Total Graded");
    m.insert("grades.graded_prefix", "Graded: ");

    // ==================== CLASSES ====================
    m.insert("classes.title", "Classes");
    m.insert("classes.my_classes", "My Classes");
    m.insert("classes.all_classes", "All Classes");
    m.insert("classes.create_class", "Create Class");
    m.insert("classes.class_name", "Class Name");
    m.insert("classes.subject", "Subject");
    m.insert("classes.teacher", "Teacher");
    m.insert("classes.students", "Students");
    m.insert("classes.schedule", "Schedule");
    m.insert("classes.term", "Term");
    m.insert("classes.no_classes", "No classes found");
    m.insert("classes.failed_load", "Failed to load classes");
    m.insert("classes.enrolled", "Enrolled");
    m.insert("classes.progress", "Progress");
    m.insert("classes.tasks", "Tasks");
    m.insert("classes.materials", "Materials");
    m.insert(
        "classes.view_description",
        "View your enrolled classes and class materials",
    );
    m.insert(
        "classes.not_enrolled",
        "You haven't been enrolled in any classes yet.",
    );
    m.insert("classes.with_teacher_prefix", "with ");

    // ==================== ASSIGNMENTS ====================
    m.insert("assignments.title", "Assignments");
    m.insert("assignments.create", "Create Assignment");
    m.insert("assignments.due_date", "Due Date");
    m.insert("assignments.submitted", "Submitted");
    m.insert("assignments.pending", "Pending");
    m.insert("assignments.overdue", "Overdue");
    m.insert("assignments.completed", "Completed");
    m.insert("assignments.grading", "Grading");
    m.insert("assignments.submit_work", "Submit Work");
    m.insert("assignments.view_submission", "View Submission");
    m.insert("assignments.no_assignments", "No assignments found");
    m.insert("assignments.instruction", "Instructions");
    m.insert("assignments.attachments", "Attachments");
    m.insert("assignments.your_work", "Your Work");
    m.insert("assignments.feedback", "Feedback");
    m.insert("assignments.loading", "Loading assignments...");
    m.insert(
        "assignments.no_class_assignments",
        "No assignments for this class yet",
    );
    m.insert("assignments.due_prefix", "Due: ");
    m.insert(
        "assignments.description",
        "View and submit your assignments",
    );
    m.insert("assignments.filter.all", "All Assignments");
    m.insert("assignments.loading_failed", "Failed to load assignments");
    m.insert("assignments.empty.all", "No Assignments Yet");
    m.insert("assignments.empty.filtered", "No {0} Assignments");
    m.insert(
        "assignments.empty.check_back",
        "Check back later for new assignments",
    );
    m.insert("assignments.action.start", "Start Assignment");
    m.insert("assignments.action.view_feedback", "View Feedback");
    m.insert("assignments.action.save_draft", "Save Draft & Close");
    m.insert("assignments.action.submit", "Submit Assignment");
    m.insert("assignments.action.submitting", "Submitting...");
    m.insert("assignments.points", " points");
    m.insert("assignments.status_prefix", "Status: ");
    m.insert(
        "assignments.personalization.info",
        "This assignment has been personalized based on your unique talents and learning style.",
    );
    m.insert("assignments.personalization.badge", "Personalized for You");
    m.insert(
        "assignments.personalization.details_title",
        "Personalization Details",
    );
    m.insert(
        "assignments.personalization.details",
        "Personalization Details",
    );
    m.insert("assignments.personalization.difficulty", "Difficulty: ");
    m.insert("assignments.personalization.est_time", "Est. Time: ");
    m.insert("assignments.work.title", "Work on Assignment");
    m.insert(
        "assignments.work.placeholder",
        "Write your assignment response here...",
    );
    m.insert("assignments.work.characters", " characters");
    m.insert(
        "assignments.work.empty_error",
        "Please write something before submitting",
    );
    m.insert("assignments.work.submit_error", "Failed to submit");
    m.insert("assignments.details.not_found", "Assignment not found");
    m.insert(
        "assignments.ai_personalizing.title",
        "AI is Customizing Your Assignment",
    );
    m.insert("assignments.ai_personalizing.description", "Please wait while our AI tailors this assignment to your unique talents and learning style...");

    // ==================== SUBMISSIONS ====================
    m.insert("submissions.title", "Submissions");
    m.insert("submissions.grade_submission", "Grade Submission");
    m.insert("submissions.enter_grade", "Enter Grade");
    m.insert("submissions.feedback_optional", "Feedback (optional)");
    m.insert("submissions.submit_grade", "Submit Grade");
    m.insert("submissions.graded", "Graded");
    m.insert("submissions.not_graded", "Not Graded");
    m.insert("submissions.submitted_at", "Submitted at");
    m.insert("submissions.no_submissions", "No submissions found");
    m.insert("submissions.grade_label", "Grade");

    // ==================== STUDENTS ====================
    m.insert("students.title", "Students");
    m.insert("students.all_students", "All Students");
    m.insert("students.student_name", "Student Name");
    m.insert("students.grade_level", "Grade Level");
    m.insert("students.enrolled_classes", "Enrolled Classes");
    m.insert("students.view_profile", "View Profile");
    m.insert("students.view_grades", "View Grades");
    m.insert("students.no_students", "No students found");
    m.insert("students.loading", "Loading students...");
    m.insert("students.failed_load", "Failed to load students");
    m.insert(
        "students.no_enrolled_class",
        "No students enrolled in this class",
    );
    m.insert("students.total", "Total Students");
    m.insert("students.submitted_count", "Submitted: ");
    m.insert("students.graded_count", "Graded: ");

    // ==================== TEACHERS ====================
    m.insert("students.title", "Students");
    m.insert("students.all_students", "All Students");
    m.insert("students.student_name", "Student Name");
    m.insert("students.grade_level", "Grade Level");
    m.insert("students.enrolled_classes", "Enrolled Classes");
    m.insert("students.view_profile", "View Profile");
    m.insert("students.view_grades", "View Grades");
    m.insert("students.no_students", "No students found");

    // ==================== TEACHERS ====================
    m.insert("teachers.title", "Teachers");
    m.insert("teachers.all_teachers", "All Teachers");
    m.insert("teachers.department", "Department");
    m.insert("teachers.no_teachers", "No teachers found");
    m.insert(
        "teachers.dashboard.no_assignments_created",
        "No assignments created yet",
    );
    m.insert(
        "teachers.dashboard.create_first_assignment",
        "Create your first assignment to get started",
    );
    m.insert(
        "teachers.dashboard.no_classes_assigned",
        "No classes assigned",
    );
    m.insert("teachers.dashboard.course_progress", "Course Progress");
    m.insert("teachers.status.active", "Active");
    m.insert("teachers.status.enrolled", "Enrolled");
    m.insert("teachers.status.to_review", "To Review");
    m.insert(
        "teachers.quick_actions.create_assignment_desc",
        "New task for your class",
    );
    m.insert(
        "teachers.quick_actions.grade_submissions",
        "Grade Submissions",
    );
    m.insert(
        "teachers.quick_actions.grade_submissions_desc",
        "Review and grade work",
    );
    m.insert(
        "teachers.quick_actions.schedule_lecture",
        "Schedule Lecture",
    );
    m.insert(
        "teachers.quick_actions.schedule_lecture_desc",
        "Plan your next session",
    );
    m.insert(
        "teachers.classes.manage_description",
        "Manage your classes and track student progress",
    );
    m.insert("teachers.classes.no_classes_yet", "No classes yet");
    m.insert(
        "teachers.classes.no_classes_assigned_desc",
        "You haven't been assigned to any classes.",
    );
    m.insert("teachers.classes.enrolled_suffix", " students enrolled");
    m.insert("teachers.classes.actions.grading", "Grading");
    m.insert("teachers.classes.modal.overview_suffix", " - Overview");
    m.insert(
        "teachers.classes.enrolled_students_label",
        "Enrolled Students",
    );
    m.insert("teachers.classes.modal.students_suffix", " - Students");
    m.insert("teachers.classes.modal.grading_suffix", " - Grading");
    m.insert("teachers.classes.assignments.status.draft", "Draft");
    m.insert("teachers.classes.assignments.to_grade_suffix", " to grade");
    m.insert(
        "teachers.classes.assignments.total_assigned",
        "Total assigned: ",
    );
    m.insert(
        "teachers.assignments.manage_description",
        "Create and manage assignments for your classes",
    );
    m.insert(
        "teachers.assignments.delete_success",
        "Assignment deleted successfully",
    );
    m.insert("teachers.assignments.delete_failed", "Failed to delete: ");
    m.insert("teachers.assignments.create_new", "Create New Assignment");
    m.insert(
        "teachers.assignments.no_assignments_title",
        "No Assignments Yet",
    );
    m.insert(
        "teachers.assignments.no_assignments_desc",
        "Create your first assignment to get started",
    );
    m.insert(
        "teachers.assignments.submission_progress",
        "Submission Progress",
    );
    m.insert("teachers.assignments.delete_tooltip", "Delete assignment");
    m.insert(
        "teachers.assignments.create.success",
        "Assignment created successfully!",
    );
    m.insert("teachers.assignments.create.failed", "Failed to create: ");
    m.insert(
        "teachers.assignments.create.title_label",
        "Assignment Title *",
    );
    m.insert(
        "teachers.assignments.create.title_placeholder",
        "e.g., Chapter 5 Quiz",
    );
    m.insert("teachers.assignments.create.class_label", "Class *");
    m.insert(
        "teachers.assignments.create.select_class",
        "Select a class...",
    );
    m.insert(
        "teachers.assignments.create.loading_classes",
        "Loading classes...",
    );
    m.insert("teachers.assignments.create.due_date_label", "Due Date *");
    m.insert(
        "teachers.assignments.create.description_label",
        "Assignment Description *",
    );
    m.insert(
        "teachers.assignments.create.description_placeholder",
        "Describe the assignment requirements...",
    );
    m.insert(
        "teachers.assignments.create.materials_label",
        "Reference Materials (for AI context)",
    );
    m.insert(
        "teachers.assignments.create.materials_selected",
        "material(s) selected",
    );
    m.insert("teachers.assignments.create.ai_title", "AI Personalization");
    m.insert("teachers.assignments.create.ai_desc", "After publishing, you can personalize this assignment for each student using AI. The system will customize the assignment based on each student's talents and learning style.");
    m.insert("teachers.assignments.create.creating_btn", "Creating...");
    m.insert(
        "teachers.assignments.create.create_btn",
        "Create Assignment",
    );
    m.insert("teachers.assignments.details.title", "Assignment Details");
    m.insert(
        "teachers.assignments.publish.success",
        "Assignment published successfully!",
    );
    m.insert("teachers.assignments.publish.failed", "Failed to publish: ");
    m.insert(
        "teachers.assignments.details.not_found",
        "Assignment not found",
    );
    m.insert(
        "teachers.assignments.details.failed_load",
        "Failed to load assignment",
    );
    m.insert("teachers.assignments.details.created_label", "Created");
    m.insert(
        "teachers.assignments.details.publish_btn",
        "Publish Assignment",
    );
    m.insert(
        "teachers.assignments.validation.required_fields",
        "Please fill in all required fields",
    );
    m.insert(
        "teachers.assignments.validation.invalid_date",
        "Invalid date format",
    );

    // ==================== SCHOOL MANAGER ====================
    m.insert("school_manager.access_denied", "Access Denied");
    m.insert(
        "school_manager.access_denied_desc",
        "You don't have permission to access the School Manager dashboard.",
    );
    m.insert("school_manager.go_to_dashboard", "Go to Your Dashboard");

    m.insert("school_manager.recent_activity", "Recent Activity");
    m.insert(
        "school_manager.recent_activity_desc",
        "Overview of latest platform updates",
    );

    // Activity Mock Data Templates
    m.insert(
        "school_manager.activity.new_student_added",
        "New student \"{0}\" was added.",
    );
    m.insert(
        "school_manager.activity.new_student_class_added",
        "New student \"{0}\" was added to {1}.",
    );
    m.insert(
        "school_manager.activity.schedule_updated",
        "Class schedule for \"{0}\" has been updated.",
    );
    m.insert(
        "school_manager.activity.report_generated",
        "Final report for {0} has been generated.",
    );

    m.insert("school_manager.system_health", "System Health");
    m.insert("school_manager.health.database", "Database");
    m.insert("school_manager.health.api_latency", "API Latency");
    m.insert("school_manager.health.storage", "Storage");
    m.insert("school_manager.health.active_users", "Active Users");
    m.insert("school_manager.health.status.healthy", "Healthy");
    m.insert("school_manager.health.status.good", "Good");
    m.insert("school_manager.health.status.moderate", "Moderate");
    m.insert("school_manager.health.status.normal", "Normal");

    m.insert("school_manager.quick_actions.title", "Quick Actions");
    m.insert("school_manager.actions.add_user", "Add User");
    m.insert(
        "school_manager.actions.add_user_desc",
        "Create new student, teacher, or parent account.",
    );
    m.insert("school_manager.actions.create_class", "Create Class");
    m.insert(
        "school_manager.actions.create_class_desc",
        "Add new class and assign a teacher.",
    );
    m.insert("school_manager.actions.view_reports", "View Reports");
    m.insert(
        "school_manager.actions.view_reports_desc",
        "Generate and view system reports.",
    );
    m.insert("school_manager.actions.system_settings", "System Settings");
    m.insert(
        "school_manager.actions.system_settings_desc",
        "Configure system preferences.",
    );

    // ==================== SCHEDULE ====================
    m.insert("schedule.title", "Schedule");
    m.insert(
        "schedule.description",
        "View your class schedule and important dates",
    );
    m.insert("schedule.today", "Today's Schedule");
    m.insert("schedule.weekly_overview", "Weekly Overview");
    m.insert("schedule.important_dates", "Important Dates");
    m.insert("schedule.classes_today", "classes today");
    m.insert("schedule.classes_count", "classes");
    m.insert("schedule.instructor_prefix", "Instructor: ");
    m.insert("schedule.status.in_progress", "IN PROGRESS");
    m.insert("schedule.status.completed", "COMPLETED");
    m.insert("schedule.status.upcoming", "UPCOMING");

    m.insert("school_manager.actions.system_settings", "System Settings");
    m.insert(
        "school_manager.actions.system_settings_desc",
        "Configure system preferences.",
    );

    // ==================== USER MANAGEMENT ====================
    m.insert("school_manager.users.title", "User Management");
    m.insert(
        "school_manager.users.description",
        "Manage students, teachers, and parents in your institution",
    );
    m.insert("school_manager.users.summary.students", "Students");
    m.insert("school_manager.users.summary.teachers", "Teachers");
    m.insert("school_manager.users.summary.parents", "Parents");
    m.insert(
        "school_manager.users.manage_btn.students",
        "Manage Students",
    );
    m.insert(
        "school_manager.users.manage_btn.teachers",
        "Manage Teachers",
    );
    m.insert("school_manager.users.manage_btn.parents", "Manage Parents");

    m.insert("school_manager.users.tabs.directory", "Directory");
    m.insert("school_manager.users.tabs.requests", "Change Requests");

    m.insert("school_manager.users.actions.add_user", "Add User");
    m.insert("school_manager.users.actions.bulk_import", "Bulk Import");
    m.insert("school_manager.users.actions.export_users", "Export Users");

    m.insert("school_manager.users.directory.title", "User Directory");
    m.insert(
        "school_manager.users.directory.search_placeholder",
        "Search users...",
    );
    m.insert("school_manager.users.directory.all_roles", "All Roles");
    m.insert("school_manager.users.directory.all_status", "All Status");
    m.insert("school_manager.users.directory.active", "Active");
    m.insert("school_manager.users.directory.inactive", "Inactive");

    m.insert("school_manager.users.table.name", "Name");
    m.insert("school_manager.users.table.role", "Role");
    m.insert("school_manager.users.table.status", "Status");
    m.insert("school_manager.users.table.joined", "Joined");
    m.insert("school_manager.users.table.actions", "Actions");

    m.insert("school_manager.users.actions.edit", "Edit");
    m.insert("school_manager.users.actions.deactivate", "Deactivate");
    m.insert("school_manager.users.actions.reactivate", "Reactivate");

    m.insert(
        "school_manager.users.messages.deactivate_success",
        "User deactivated successfully",
    );
    m.insert(
        "school_manager.users.messages.deactivate_fail",
        "Failed to deactivate user: ",
    );
    m.insert(
        "school_manager.users.messages.reactivate_success",
        "User reactivated successfully",
    );
    m.insert(
        "school_manager.users.messages.reactivate_fail",
        "Failed to reactivate user: ",
    );
    m.insert(
        "school_manager.users.messages.update_success",
        "User updated successfully",
    );
    m.insert(
        "school_manager.users.messages.update_fail",
        "Failed to update user: ",
    );
    m.insert(
        "school_manager.users.messages.load_error",
        "Error loading users: {e}",
    );

    m.insert("school_manager.users.edit_modal.title", "Edit User");
    m.insert("school_manager.users.edit_modal.saving", "Saving...");
    m.insert("school_manager.users.edit_modal.save", "Save Changes");

    m.insert(
        "school_manager.users.import_modal.title",
        "Bulk Import Users",
    );
    m.insert(
        "school_manager.users.import_modal.csv_title",
        "CSV Format Required",
    );
    m.insert(
        "school_manager.users.import_modal.csv_desc",
        "Upload a CSV file with columns: name, email, role (student/teacher/parent)",
    );
    m.insert(
        "school_manager.users.import_modal.drop_text",
        "Drop your CSV file here",
    );
    m.insert(
        "school_manager.users.import_modal.browse_text",
        "or click to browse files",
    );
    m.insert(
        "school_manager.users.import_modal.coming_soon",
        "Full bulk import functionality coming soon. For now, please add users individually.",
    );
    m.insert("school_manager.users.import_modal.import_btn", "Import");

    m.insert("school_manager.users.export_modal.title", "Export Users");
    m.insert(
        "school_manager.users.export_modal.format_label",
        "Export Format",
    );
    m.insert("school_manager.users.export_modal.options_label", "Options");
    m.insert(
        "school_manager.users.export_modal.include_inactive",
        "Include inactive users",
    );
    m.insert("school_manager.users.export_modal.coming_soon", "Export functionality coming soon. You'll be able to download user data for backup or migration.");
    m.insert("school_manager.users.export_modal.export_btn", "Export");

    // ==================== CLASS MANAGEMENT ====================
    m.insert("school_manager.classes.title", "Class Management");
    m.insert(
        "school_manager.classes.description",
        "Manage classes, courses, and academic programs",
    );
    m.insert("school_manager.classes.active_classes", "Active Classes");
    m.insert("school_manager.classes.actions.new_class", "New Class");

    m.insert("school_manager.classes.empty.title", "No classes yet");
    m.insert(
        "school_manager.classes.empty.desc",
        "Create your first class to get started managing courses and students.",
    );
    m.insert(
        "school_manager.classes.error.load_failed",
        "Failed to load classes",
    );

    m.insert(
        "school_manager.classes.create_modal.title",
        "Create New Class",
    );
    m.insert(
        "school_manager.classes.create_modal.class_name",
        "Class Name",
    );
    m.insert(
        "school_manager.classes.create_modal.class_name_placeholder",
        "e.g., Math 101 - Section A",
    );
    m.insert(
        "school_manager.classes.create_modal.term_placeholder",
        "e.g., Fall 2024",
    );
    m.insert(
        "school_manager.classes.create_modal.create_btn",
        "Create Class",
    );
    m.insert(
        "school_manager.classes.create_modal.creating",
        "Creating...",
    );

    m.insert(
        "school_manager.classes.detail_modal.title",
        "{class} - Manage Students",
    );
    m.insert(
        "school_manager.classes.detail_modal.add_student",
        "Add Student",
    );
    m.insert(
        "school_manager.classes.detail_modal.select_student",
        "Select a student...",
    );
    m.insert(
        "school_manager.classes.detail_modal.error_loading_students",
        "Error loading students",
    );
    m.insert(
        "school_manager.classes.detail_modal.enrolled_students",
        "Enrolled Students",
    );
    m.insert(
        "school_manager.classes.detail_modal.no_students",
        "No students enrolled yet",
    );
    m.insert(
        "school_manager.classes.detail_modal.failed_load_students",
        "Failed to load students",
    );
    m.insert("school_manager.classes.detail_modal.enroll_btn", "Enroll");

    m.insert(
        "school_manager.classes.errors.name_required",
        "Class name is required",
    );
    m.insert(
        "school_manager.classes.errors.subject_required",
        "Please select a subject",
    );
    m.insert(
        "school_manager.classes.errors.term_required",
        "Term is required",
    );
    m.insert(
        "school_manager.classes.errors.select_student_required",
        "Please select a student",
    );
    m.insert(
        "school_manager.classes.errors.enroll_failed",
        "Failed to enroll: ",
    );
    m.insert(
        "school_manager.classes.errors.remove_failed",
        "Failed to remove: ",
    );

    // ==================== REPORTS ====================
    m.insert("school_manager.reports.title", "Reports & Analytics");
    m.insert(
        "school_manager.reports.description",
        "Comprehensive insights and reports about your institution",
    );
    m.insert(
        "school_manager.reports.config.title",
        "Report Configuration",
    );
    m.insert("school_manager.reports.config.export", "📥 Export");
    m.insert(
        "school_manager.reports.config.generate",
        "📊 Generate Report",
    );

    m.insert(
        "school_manager.reports.types.class_performance",
        "Class Performance",
    );
    m.insert(
        "school_manager.reports.types.class_performance_desc",
        "Student grades and class stats",
    );
    m.insert(
        "school_manager.reports.types.teacher_workload",
        "Teacher Workload",
    );
    m.insert(
        "school_manager.reports.types.teacher_workload_desc",
        "Assignments and hours",
    );
    m.insert(
        "school_manager.reports.types.attendance",
        "Student Attendance",
    );
    m.insert(
        "school_manager.reports.types.attendance_desc",
        "Attendance records",
    );
    m.insert(
        "school_manager.reports.types.parent_engagement",
        "Parent Engagement",
    );
    m.insert(
        "school_manager.reports.types.parent_engagement_desc",
        "Portal activity metrics",
    );

    m.insert(
        "school_manager.reports.filters.class_label",
        "Class/Subject",
    );
    m.insert("school_manager.reports.filters.all_classes", "All Classes");
    m.insert("school_manager.reports.filters.teacher_label", "Teacher");
    m.insert(
        "school_manager.reports.filters.all_teachers",
        "All Teachers",
    );
    m.insert(
        "school_manager.reports.filters.student_label",
        "Student (Optional)",
    );
    m.insert(
        "school_manager.reports.filters.all_students",
        "All Students",
    );
    m.insert(
        "school_manager.reports.filters.date_range_label",
        "Date Range",
    );

    m.insert(
        "school_manager.reports.filters.ranges.this_week",
        "This Week",
    );
    m.insert(
        "school_manager.reports.filters.ranges.this_month",
        "This Month",
    );
    m.insert(
        "school_manager.reports.filters.ranges.this_semester",
        "This Semester",
    );
    m.insert(
        "school_manager.reports.filters.ranges.this_year",
        "This Year",
    );
    m.insert(
        "school_manager.reports.filters.ranges.custom",
        "Custom Range",
    );

    m.insert(
        "school_manager.reports.class_performance.title",
        "Class Performance Report",
    );
    m.insert(
        "school_manager.reports.class_performance.subtitle_all",
        "All Classes • {date}",
    );
    m.insert(
        "school_manager.reports.class_performance.subtitle_filtered",
        "{filter} • {date}",
    );
    m.insert(
        "school_manager.reports.class_performance.export_pdf",
        "📥 PDF",
    );
    m.insert(
        "school_manager.reports.class_performance.export_excel",
        "📊 Excel",
    );

    m.insert(
        "school_manager.reports.stats.total_reports",
        "Total Reports",
    );
    m.insert("school_manager.reports.stats.available", "Available");
    m.insert("school_manager.reports.stats.no_data", "No data");
    m.insert("school_manager.reports.stats.students", "Students");
    m.insert("school_manager.reports.stats.tracked", "Tracked");
    m.insert("school_manager.reports.stats.teachers", "Teachers");
    m.insert("school_manager.reports.stats.active", "Active");
    m.insert("school_manager.reports.stats.date_range", "Date Range");
    m.insert("school_manager.reports.stats.selected", "Selected");

    m.insert(
        "school_manager.reports.chart.title",
        "Performance Chart Visualization",
    );
    m.insert(
        "school_manager.reports.chart.desc",
        "Grade distribution and trend analysis (Next Phase)",
    );

    m.insert("school_manager.reports.table.title", "Reports Details");
    m.insert("school_manager.reports.table.student", "Student");
    m.insert("school_manager.reports.table.teacher", "Teacher");
    m.insert("school_manager.reports.table.email", "Student Email");
    m.insert("school_manager.reports.table.summary", "AI Summary");
    m.insert("school_manager.reports.table.created", "Created");
    m.insert(
        "school_manager.reports.table.empty",
        "No reports found for the selected filters",
    );
    m.insert("school_manager.reports.table.loading", "Loading reports...");
    m.insert("school_manager.reports.table.unknown_student", "Unknown");
    m.insert(
        "school_manager.reports.table.unassigned_teacher",
        "Not assigned",
    );
    m.insert("school_manager.reports.table.no_summary", "No summary");

    m.insert(
        "school_manager.reports.sidebar.summary_title",
        "Report Summary",
    );
    m.insert("school_manager.reports.sidebar.type_label", "Report Type:");
    m.insert("school_manager.reports.sidebar.period_label", "Period:");
    m.insert(
        "school_manager.reports.sidebar.generated_label",
        "Generated:",
    );
    m.insert("school_manager.reports.sidebar.just_now", "Just now");

    m.insert(
        "school_manager.reports.sidebar.export_title",
        "Export Options",
    );
    m.insert(
        "school_manager.reports.sidebar.export_pdf",
        "📄 Export as PDF",
    );
    m.insert(
        "school_manager.reports.sidebar.export_excel",
        "📊 Export as Excel",
    );
    m.insert(
        "school_manager.reports.sidebar.export_csv",
        "📑 Export as CSV",
    );
    m.insert(
        "school_manager.reports.sidebar.export_image",
        "🖼️ Export as Image",
    );

    m.insert(
        "school_manager.reports.sidebar.schedule_title",
        "Schedule Reports",
    );
    m.insert(
        "school_manager.reports.sidebar.schedule_weekly",
        "⏰ Schedule Weekly",
    );
    m.insert(
        "school_manager.reports.sidebar.schedule_monthly",
        "📅 Schedule Monthly",
    );
    m.insert(
        "school_manager.reports.sidebar.schedule_quarterly",
        "📆 Schedule Quarterly",
    );

    m.insert(
        "school_manager.reports.workload.title",
        "Teacher Workload Report",
    );
    m.insert(
        "school_manager.reports.workload.analysis",
        "Teacher Workload Analysis",
    );
    m.insert(
        "school_manager.reports.workload.desc",
        "Teaching hours, class assignments, and workload distribution (Next Phase)",
    );

    m.insert(
        "school_manager.reports.attendance.title",
        "Student Attendance Report",
    );
    m.insert(
        "school_manager.reports.attendance.analytics",
        "Attendance Analytics",
    );
    m.insert(
        "school_manager.reports.attendance.desc",
        "Attendance patterns, trends, and exception reports (Next Phase)",
    );

    m.insert(
        "school_manager.reports.engagement.title",
        "Parent Engagement Report",
    );
    m.insert(
        "school_manager.reports.engagement.analytics",
        "Parent Portal Analytics",
    );
    m.insert(
        "school_manager.reports.engagement.desc",
        "Login frequency, feature usage, and engagement metrics (Next Phase)",
    );

    // Dashboard Activity
    m.insert(
        "school_manager.activity.new_student_added",
        "New student \"{0}\" was added.",
    );

    // Requests
    m.insert(
        "school_manager.requests.title",
        "Pending Profile Change Requests",
    );
    m.insert(
        "school_manager.requests.no_auth_token",
        "No auth token found",
    );
    m.insert(
        "school_manager.requests.success",
        "Request {0} successfully",
    );
    m.insert(
        "school_manager.requests.failure",
        "Failed to decide request: {0}",
    );
    m.insert("school_manager.requests.empty", "No pending requests");
    m.insert("school_manager.requests.requested_by", "Requested by: {0}");
    m.insert("school_manager.requests.reject", "Reject");
    m.insert("school_manager.requests.approve", "Approve");
    m.insert(
        "school_manager.requests.error",
        "Error loading requests: {0}",
    );
    m.insert("school_manager.requests.loading", "Loading requests...");

    // User Creation Hub
    m.insert("school_manager.users.creation.title", "User Creation Hub");
    m.insert(
        "school_manager.users.creation.subtitle",
        "Create and manage student, teacher, and parent accounts",
    );
    m.insert("school_manager.users.creation.cancel", "Cancel");
    m.insert("school_manager.users.creation.import", "Bulk Import");

    // System Health
    m.insert("school_manager.system_health", "System Health");
    m.insert("school_manager.health.database", "Database");
    m.insert("school_manager.health.api_latency", "API Latency");
    m.insert("school_manager.health.storage", "Storage");
    m.insert("school_manager.health.active_users", "Active Users");
    m.insert("school_manager.health.status.healthy", "Healthy");
    m.insert("school_manager.health.status.good", "Good");
    m.insert("school_manager.health.status.moderate", "Moderate");
    m.insert("school_manager.health.status.normal", "Normal");

    m.insert("school_manager.users.creation.tabs.students", "Students");
    m.insert("school_manager.users.creation.tabs.teachers", "Teachers");
    m.insert("school_manager.users.creation.tabs.parents", "Parents");

    m.insert(
        "school_manager.users.creation.personal_info",
        "Personal Information",
    );
    m.insert(
        "school_manager.users.creation.academic_info",
        "Academic Information",
    );
    m.insert(
        "school_manager.users.creation.professional_info",
        "Professional Information",
    );
    m.insert(
        "school_manager.users.creation.class_assignment",
        "Class Assignment",
    );
    m.insert(
        "school_manager.users.creation.student_association",
        "Student Association",
    );
    m.insert("school_manager.users.creation.creating", "Creating...");

    m.insert("school_manager.users.creation.first_name", "First Name *");
    m.insert("school_manager.users.creation.last_name", "Last Name *");
    m.insert("school_manager.users.creation.full_name", "Full Name *");
    m.insert("school_manager.users.creation.email", "Email Address *");
    m.insert("school_manager.users.creation.phone", "Phone Number");
    m.insert("school_manager.users.creation.dob", "Date of Birth *");
    m.insert("school_manager.users.creation.student_id", "Student ID *");
    m.insert("school_manager.users.creation.grade_level", "Grade Level *");
    m.insert(
        "school_manager.users.creation.enrollment_date",
        "Enrollment Date *",
    );
    m.insert(
        "school_manager.users.creation.class_section",
        "Class Section",
    );
    m.insert(
        "school_manager.users.creation.academic_year",
        "Academic Year *",
    );
    m.insert("school_manager.users.creation.employee_id", "Employee ID *");
    m.insert("school_manager.users.creation.department", "Department *");
    m.insert("school_manager.users.creation.subjects", "Subjects *");
    m.insert("school_manager.users.creation.hire_date", "Hire Date *");
    m.insert(
        "school_manager.users.creation.qualifications",
        "Qualifications & Certifications",
    );
    m.insert(
        "school_manager.users.creation.assign_classes",
        "Assign Classes",
    );
    m.insert("school_manager.users.creation.parent_id", "Parent ID *");
    m.insert(
        "school_manager.users.creation.relationship",
        "Relationship *",
    );
    m.insert(
        "school_manager.users.creation.associated_students",
        "Associated Students *",
    );

    // Dropdown Options
    m.insert("school_manager.users.creation.grades.9", "9th Grade");
    m.insert("school_manager.users.creation.grades.10", "10th Grade");
    m.insert("school_manager.users.creation.grades.11", "11th Grade");
    m.insert("school_manager.users.creation.grades.12", "12th Grade");

    m.insert("school_manager.users.creation.sections.a", "Section A");
    m.insert("school_manager.users.creation.sections.b", "Section B");
    m.insert("school_manager.users.creation.sections.c", "Section C");

    m.insert("school_manager.users.creation.subjects.math", "Mathematics");
    m.insert("school_manager.users.creation.subjects.physics", "Physics");
    m.insert(
        "school_manager.users.creation.subjects.chemistry",
        "Chemistry",
    );
    m.insert("school_manager.users.creation.subjects.biology", "Biology");
    m.insert(
        "school_manager.users.creation.subjects.english",
        "English Literature",
    );
    m.insert("school_manager.users.creation.subjects.history", "History");
    m.insert(
        "school_manager.users.creation.subjects.cs",
        "Computer Science",
    );

    m.insert(
        "school_manager.users.creation.class_assignment_help",
        "Select classes to assign to this teacher. Hold Ctrl/Cmd to select multiple.",
    );
    m.insert("school_manager.users.creation.student_association_help", "Select one or more students associated with this parent. Hold Ctrl/Cmd to select multiple.");

    m.insert(
        "school_manager.users.creation.placeholders.first_name",
        "Enter first name",
    );
    m.insert(
        "school_manager.users.creation.placeholders.last_name",
        "Enter last name",
    );
    m.insert(
        "school_manager.users.creation.placeholders.full_name",
        "Enter full name",
    );
    m.insert(
        "school_manager.users.creation.placeholders.relationship",
        "Enter relationship (e.g., Father)",
    );

    // Sidebar Stats & Tips - Students
    m.insert(
        "school_manager.users.creation.stats.student.total",
        "Total Students",
    );
    m.insert(
        "school_manager.users.creation.stats.student.new",
        "New This Week",
    );
    m.insert(
        "school_manager.users.creation.stats.student.pending",
        "Pending Approval",
    );
    m.insert(
        "school_manager.users.creation.stats.student.total_change",
        "+12 this month",
    );
    m.insert(
        "school_manager.users.creation.stats.student.new_change",
        "+3 vs last week",
    );
    m.insert(
        "school_manager.users.creation.stats.student.pending_change",
        "Need review",
    );

    m.insert(
        "school_manager.users.creation.tips.student.id",
        "Student IDs should follow the STU format (STU123456)",
    );
    m.insert(
        "school_manager.users.creation.tips.student.email",
        "Welcome emails are sent automatically to new students",
    );
    m.insert(
        "school_manager.users.creation.tips.student.parent",
        "Parent association is required for student accounts",
    );

    // Sidebar Stats & Tips - Teachers
    m.insert(
        "school_manager.users.creation.stats.teacher.total",
        "Total Teachers",
    );
    m.insert(
        "school_manager.users.creation.stats.teacher.active",
        "Active Classes",
    );
    m.insert(
        "school_manager.users.creation.stats.teacher.avg",
        "Avg Students/Class",
    );
    m.insert(
        "school_manager.users.creation.stats.teacher.total_change",
        "+2 this month",
    );
    m.insert(
        "school_manager.users.creation.stats.teacher.active_change",
        "All assigned",
    );
    m.insert(
        "school_manager.users.creation.stats.teacher.avg_change",
        "Optimal range",
    );

    m.insert(
        "school_manager.users.creation.tips.teacher.subjects",
        "Teachers can teach multiple subjects and grades",
    );
    m.insert(
        "school_manager.users.creation.tips.teacher.cert",
        "Certifications should be current and verifiable",
    );
    m.insert(
        "school_manager.users.creation.tips.teacher.assign",
        "Class assignments are made after account creation",
    );

    // Sidebar Stats & Tips - Parents
    m.insert(
        "school_manager.users.creation.stats.parent.total",
        "Total Parents",
    );
    m.insert(
        "school_manager.users.creation.stats.parent.linked",
        "Linked Students",
    );
    m.insert(
        "school_manager.users.creation.stats.parent.engagement",
        "Engagement Rate",
    );
    m.insert(
        "school_manager.users.creation.stats.parent.total_change",
        "+8 this month",
    );
    m.insert(
        "school_manager.users.creation.stats.parent.linked_change",
        "Some parents have multiple",
    );
    m.insert(
        "school_manager.users.creation.stats.parent.engagement_change",
        "Excellent",
    );

    m.insert(
        "school_manager.users.creation.tips.parent.multiple",
        "Each parent can be linked to multiple students",
    );
    m.insert(
        "school_manager.users.creation.tips.parent.access",
        "Parents have access to their children's academic progress",
    );
    m.insert(
        "school_manager.users.creation.tips.parent.mobile",
        "Parent accounts receive mobile notifications for important updates",
    );

    // Activity
    m.insert(
        "school_manager.users.creation.activity.student.created",
        "John Smith - Student account created",
    );
    m.insert(
        "school_manager.users.creation.activity.teacher.created",
        "Ms. Johnson - Teacher account created",
    );
    m.insert(
        "school_manager.users.creation.activity.parent.created",
        "Mary Davis - Parent account created",
    );
    m.insert(
        "school_manager.users.creation.activity.student.email",
        "Welcome emails sent to 5 new students",
    );
    m.insert(
        "school_manager.users.creation.activity.teacher.updated",
        "Class assignments updated for 3 teachers",
    );
    m.insert(
        "school_manager.users.creation.activity.parent.access",
        "Parent portal access granted to 2 parents",
    );
    m.insert(
        "school_manager.users.creation.activity.time.2h",
        "2 hours ago",
    );
    m.insert(
        "school_manager.users.creation.activity.time.5h",
        "5 hours ago",
    );

    // Parent Children Section
    m.insert("parent.children.title", "My Children");
    m.insert(
        "parent.children.desc",
        "Detailed information about your children's academic progress",
    );
    m.insert("parent.children.error", "Failed to load children: {0}");
    m.insert("parent.children.empty.title", "No children linked");
    m.insert(
        "parent.children.empty.desc",
        "Contact school administration to link your children to your account.",
    );

    m.insert("parent.children.actions.view_grades", "View Grades");
    m.insert("parent.children.actions.attendance", "Attendance");
    m.insert("parent.children.actions.message_teacher", "Message Teacher");
    m.insert("parent.children.actions.assignments", "Assignments");

    m.insert("parent.children.grades.current_gpa", "Current GPA");
    m.insert("parent.children.grades.loading", "Loading grades...");
    m.insert("parent.children.grades.failed", "Failed to load: {0}");
    m.insert("parent.children.grades.empty", "No graded assignments yet");

    m.insert(
        "parent.children.attendance.loading",
        "Loading attendance...",
    );
    m.insert("parent.children.attendance.failed", "Failed to load: {0}");
    m.insert("parent.children.attendance.present", "Present");
    m.insert("parent.children.attendance.absent", "Absent");
    m.insert("parent.children.attendance.rate", "Rate");
    m.insert(
        "parent.children.attendance.recent_absences",
        "Recent Absences",
    );

    m.insert(
        "parent.children.assignments.loading",
        "Loading assignments...",
    );
    m.insert("parent.children.assignments.failed", "Failed to load: {0}");
    m.insert("parent.children.assignments.empty", "No assignments found");
    m.insert("parent.children.assignments.due", "Due: {0}");

    // Parent Communication Section
    m.insert("parent.communication.title", "Communications");
    m.insert(
        "parent.communication.desc",
        "Messages and updates from teachers and school administration",
    );
    m.insert("parent.communication.compose.title", "Send New Message");
    m.insert("parent.communication.compose.to", "To:");
    m.insert("parent.communication.compose.child", "Child:");
    m.insert("parent.communication.compose.subject", "Subject:");
    m.insert(
        "parent.communication.compose.subject_ph",
        "Enter subject...",
    );
    m.insert("parent.communication.compose.message", "Message:");
    m.insert(
        "parent.communication.compose.message_ph",
        "Type your message here...",
    );
    m.insert("parent.communication.compose.send", "Send Message");
    m.insert(
        "parent.communication.compose.options.all_teachers",
        "All Teachers",
    );

    m.insert("parent.communication.messages.title", "Recent Messages");
    m.insert("parent.communication.messages.re", "Re: {0}");
    m.insert("parent.communication.messages.reply", "Reply");
    m.insert("parent.communication.messages.archive", "Archive");

    m.insert("parent.communication.messages.reply", "Reply");
    m.insert("parent.communication.messages.archive", "Archive");

    // Parent Reports Section
    m.insert("parent.reports.title", "Reports & Analytics");
    m.insert(
        "parent.reports.desc",
        "View detailed reports and analytics for your children",
    );
    m.insert("parent.reports.filters.title", "Generate Custom Report");
    m.insert("parent.reports.filters.child", "Child:");
    m.insert("parent.reports.filters.type", "Report Type:");
    m.insert("parent.reports.filters.period", "Time Period:");
    m.insert("parent.reports.filters.generate", "Generate Report");

    m.insert(
        "parent.reports.filters.options.all_children",
        "All Children",
    );
    m.insert(
        "parent.reports.filters.options.academic",
        "Academic Performance",
    );
    m.insert(
        "parent.reports.filters.options.attendance",
        "Attendance Report",
    );
    m.insert("parent.reports.filters.options.behavior", "Behavior Report");
    m.insert(
        "parent.reports.filters.options.comprehensive",
        "Comprehensive Report",
    );
    m.insert(
        "parent.reports.filters.options.current_semester",
        "Current Semester",
    );
    m.insert("parent.reports.filters.options.last_month", "Last Month");
    m.insert(
        "parent.reports.filters.options.last_quarter",
        "Last Quarter",
    );
    m.insert(
        "parent.reports.filters.options.academic_year",
        "Academic Year",
    );

    m.insert("parent.reports.available.title", "Available Reports");
    m.insert(
        "parent.reports.available.academic.title",
        "Academic Performance",
    );
    m.insert("parent.reports.available.academic.desc", "Comprehensive academic overview including grades, GPA trends, and subject-wise performance");
    m.insert(
        "parent.reports.available.attendance.title",
        "Attendance Report",
    );
    m.insert(
        "parent.reports.available.attendance.desc",
        "Detailed attendance records including absences, tardiness, and patterns",
    );
    m.insert("parent.reports.available.behavior.title", "Behavior Report");
    m.insert(
        "parent.reports.available.behavior.desc",
        "Behavior assessments, conduct records, and teacher feedback",
    );
    m.insert(
        "parent.reports.available.standardized.title",
        "Standardized Tests",
    );
    m.insert(
        "parent.reports.available.standardized.desc",
        "Standardized test scores and progress tracking",
    );

    m.insert("parent.reports.available.updated", "Updated: {0}");
    m.insert("parent.reports.available.for", "For: {0}");
    m.insert("parent.reports.available.download", "Download PDF");

    m.insert("parent.reports.recent.title", "Recently Downloaded");

    // Settings
    m.insert("school_manager.settings.title", "Settings & Profile");
    m.insert(
        "school_manager.settings.description",
        "Manage your account and institution settings",
    );
    m.insert("school_manager.settings.tabs.profile", "Profile");
    m.insert("school_manager.settings.tabs.security", "Security");
    m.insert("school_manager.settings.tabs.general", "General");
    m.insert(
        "school_manager.settings.tabs.notifications",
        "Notifications",
    );

    // General Settings
    m.insert("school_manager.settings.general.title", "General Settings");
    m.insert(
        "school_manager.settings.general.loading",
        "Loading preferences...",
    );
    m.insert("school_manager.settings.general.timezone", "Timezone");
    m.insert("school_manager.settings.general.language", "Language");
    m.insert("school_manager.settings.general.date_format", "Date Format");
    m.insert("school_manager.settings.general.time_format", "Time Format");
    m.insert("school_manager.settings.general.save_btn", "Save Settings");
    m.insert(
        "school_manager.settings.general.success",
        "Success! Settings saved.",
    );
    m.insert("school_manager.settings.general.error", "Error: {0}");

    // Timezone Options
    m.insert("school_manager.settings.general.timezone.utc", "UTC");
    m.insert(
        "school_manager.settings.general.timezone.et",
        "Eastern Time (ET)",
    );
    m.insert(
        "school_manager.settings.general.timezone.ct",
        "Central Time (CT)",
    );
    m.insert(
        "school_manager.settings.general.timezone.mt",
        "Mountain Time (MT)",
    );
    m.insert(
        "school_manager.settings.general.timezone.pt",
        "Pacific Time (PT)",
    );
    m.insert(
        "school_manager.settings.general.timezone.gmt",
        "London (GMT)",
    );
    m.insert(
        "school_manager.settings.general.timezone.cet",
        "Paris (CET)",
    );
    m.insert(
        "school_manager.settings.general.timezone.jst",
        "Tokyo (JST)",
    );
    m.insert(
        "school_manager.settings.general.timezone.gst",
        "Dubai (GST)",
    );
    m.insert(
        "school_manager.settings.general.timezone.aedt",
        "Sydney (AEDT)",
    );

    // Time Format Options
    m.insert(
        "school_manager.settings.general.time_format.24h",
        "24-hour (14:30)",
    );
    m.insert(
        "school_manager.settings.general.time_format.12h",
        "12-hour (2:30 PM)",
    );

    // Notification Settings
    m.insert(
        "school_manager.settings.notifications.title",
        "Notification Preferences",
    );
    m.insert(
        "school_manager.settings.notifications.loading",
        "Loading preferences...",
    );
    m.insert(
        "school_manager.settings.notifications.channels",
        "Notification Channels",
    );
    m.insert(
        "school_manager.settings.notifications.types",
        "Notification Types",
    );
    m.insert(
        "school_manager.settings.notifications.digest",
        "Email Digest",
    );
    m.insert(
        "school_manager.settings.notifications.save_btn",
        "Save Preferences",
    );
    m.insert(
        "school_manager.settings.notifications.success",
        "Success! Preferences saved.",
    );
    m.insert("school_manager.settings.notifications.error", "Error: {0}");

    m.insert(
        "school_manager.settings.notifications.email",
        "Email Notifications",
    );
    m.insert(
        "school_manager.settings.notifications.push",
        "Push Notifications",
    );
    m.insert(
        "school_manager.settings.notifications.in_app",
        "In-App Notifications",
    );

    m.insert(
        "school_manager.settings.notifications.user_reg",
        "User Registered",
    );
    m.insert(
        "school_manager.settings.notifications.user_reg_desc",
        "Notify when a new user joins the system",
    );
    m.insert(
        "school_manager.settings.notifications.class_created",
        "Class Created",
    );
    m.insert(
        "school_manager.settings.notifications.class_created_desc",
        "Notify when a new class is created",
    );
    m.insert(
        "school_manager.settings.notifications.assignment",
        "Assignment Submitted",
    );
    m.insert(
        "school_manager.settings.notifications.assignment_desc",
        "Notify when students submit assignments",
    );
    m.insert(
        "school_manager.settings.notifications.report",
        "Report Generated",
    );
    m.insert(
        "school_manager.settings.notifications.report_desc",
        "Notify when student reports are generated",
    );
    m.insert(
        "school_manager.settings.notifications.profile_change",
        "Profile Change Requests",
    );
    m.insert(
        "school_manager.settings.notifications.profile_change_desc",
        "Notify when profile change requests are submitted",
    );
    m.insert(
        "school_manager.settings.notifications.announcements",
        "System Announcements",
    );
    m.insert(
        "school_manager.settings.notifications.announcements_desc",
        "Receive important system updates",
    );

    m.insert(
        "school_manager.settings.notifications.digest.never",
        "Never",
    );
    m.insert(
        "school_manager.settings.notifications.digest.daily",
        "Daily Summary",
    );
    m.insert(
        "school_manager.settings.notifications.digest.weekly",
        "Weekly Summary",
    );

    // Security Settings
    m.insert("school_manager.settings.security.title", "Change Password");
    m.insert(
        "school_manager.settings.security.current_pwd",
        "Current Password",
    );
    m.insert("school_manager.settings.security.new_pwd", "New Password");
    m.insert(
        "school_manager.settings.security.confirm_pwd",
        "Confirm New Password",
    );
    m.insert(
        "school_manager.settings.security.update_btn",
        "Update Password",
    );
    m.insert(
        "school_manager.settings.security.mismatch",
        "New passwords do not match",
    );
    m.insert(
        "school_manager.settings.security.min_length",
        "Password must be at least 8 characters",
    );
    m.insert(
        "school_manager.settings.security.success",
        "Password changed successfully",
    );
    m.insert(
        "school_manager.settings.security.failure",
        "Failed to change password",
    );

    // Profile Settings
    m.insert(
        "school_manager.settings.profile.info_title",
        "Personal Information",
    );
    m.insert(
        "school_manager.settings.profile.loading",
        "Loading profile...",
    );
    m.insert("school_manager.settings.profile.full_name", "Full Name");
    m.insert("school_manager.settings.profile.email", "Email Address");
    m.insert("school_manager.settings.profile.phone", "Phone Number");
    m.insert("school_manager.settings.profile.office", "Office Location");
    m.insert("school_manager.settings.profile.hours", "Work Hours");
    m.insert(
        "school_manager.settings.profile.emergency",
        "Emergency Contact",
    );
    m.insert("school_manager.settings.profile.save_btn", "Save Changes");
    m.insert("school_manager.settings.profile.updated", "Profile updated");
    m.insert(
        "school_manager.settings.profile.role_admin",
        "Administrator",
    );
    m.insert(
        "school_manager.settings.profile.actions_title",
        "Profile Actions",
    );
    m.insert(
        "school_manager.settings.profile.request_change",
        "Request Profile Change",
    );
    m.insert(
        "school_manager.settings.profile.change_pwd",
        "Change Password",
    );
    m.insert(
        "school_manager.settings.profile.pwd_requirements",
        "Password requirements:",
    );
    m.insert(
        "school_manager.settings.profile.pwd_req_1",
        "At least 8 characters",
    );
    m.insert(
        "school_manager.settings.profile.pwd_req_2",
        "At least one uppercase letter",
    );
    m.insert(
        "school_manager.settings.profile.pwd_req_3",
        "At least one number",
    );
    m.insert("school_manager.settings.profile.pwd_coming_soon", "Password change functionality coming soon. Currently managed through your authentication provider.");
    m.insert(
        "school_manager.settings.profile.request_submitted",
        "Request submitted",
    );
    m.insert(
        "school_manager.settings.profile.log.updated",
        "Profile updated",
    );
    m.insert(
        "school_manager.settings.profile.log.submitted",
        "Request submitted",
    );
    m.insert(
        "school_manager.users.creation.placeholders.email_student",
        "student@school.edu",
    );
    m.insert(
        "school_manager.users.creation.placeholders.email_teacher",
        "teacher@school.edu",
    );
    m.insert(
        "school_manager.users.creation.placeholders.email_parent",
        "parent@example.com",
    );
    m.insert(
        "school_manager.users.creation.placeholders.phone",
        "(555) 123-4567",
    );
    m.insert(
        "school_manager.users.creation.placeholders.student_id",
        "STU001234",
    );
    m.insert(
        "school_manager.users.creation.placeholders.employee_id",
        "TCH001234",
    );
    m.insert(
        "school_manager.users.creation.placeholders.parent_id",
        "PAR001234",
    );
    m.insert(
        "school_manager.users.creation.placeholders.qualifications",
        "List degrees, certifications, and qualifications...",
    );

    m.insert(
        "school_manager.users.creation.btn.create_student",
        "Create Student Account",
    );
    m.insert(
        "school_manager.users.creation.btn.create_teacher",
        "Create Teacher Account",
    );
    m.insert(
        "school_manager.users.creation.btn.create_parent",
        "Create Parent Account",
    );

    m.insert(
        "school_manager.users.creation.options.select_grade",
        "Select Grade Level",
    );
    m.insert(
        "school_manager.users.creation.options.select_section",
        "Select Section",
    );
    m.insert(
        "school_manager.users.creation.options.select_dept",
        "Select Department",
    );
    m.insert(
        "school_manager.users.creation.options.loading_classes",
        "Loading classes...",
    );
    m.insert(
        "school_manager.users.creation.options.loading_students",
        "Loading students...",
    );

    m.insert(
        "school_manager.users.creation.success.student",
        "Student created successfully! Temporary password: {0}",
    );
    m.insert(
        "school_manager.users.creation.success.teacher",
        "Teacher created successfully! Temporary password: {0}",
    );
    m.insert(
        "school_manager.users.creation.success.parent",
        "Parent account created successfully. Temporary password: {0}",
    );
    m.insert(
        "school_manager.users.creation.error.parent",
        "Failed to create parent: {0}",
    );

    m.insert(
        "school_manager.users.creation.stats.title",
        "Current Statistics",
    );
    m.insert("school_manager.users.creation.tips.title", "Quick Tips");
    m.insert(
        "school_manager.users.creation.activity.title",
        "Recent Activity",
    );

    // ==================== PARENTS ====================
    m.insert("parents.title", "Parents");
    m.insert("parents.my_children", "My Children");
    m.insert("parents.child_progress", "Child Progress");
    m.insert("parents.view_report", "View Report");
    m.insert("parents.contact_teacher", "Contact Teacher");
    m.insert("parents.no_children", "No children registered");

    // ==================== SETTINGS ====================
    m.insert("settings.title", "Settings");
    m.insert("settings.general", "General Settings");
    m.insert("settings.notifications", "Notification Settings");
    m.insert("settings.security", "Security Settings");
    m.insert("settings.profile", "Profile Settings");
    m.insert("settings.timezone", "Timezone");
    m.insert("settings.date_format", "Date Format");
    m.insert("settings.time_format", "Time Format");
    m.insert("settings.save_changes", "Save Changes");
    m.insert("settings.changes_saved", "Changes saved successfully");

    // ==================== REPORTS ====================
    m.insert("reports.title", "Reports");
    m.insert("reports.generate", "Generate Report");
    m.insert("reports.class_performance", "Class Performance");
    m.insert("reports.student_progress", "Student Progress");
    m.insert("reports.attendance_report", "Attendance Report");
    m.insert("reports.grade_distribution", "Grade Distribution");
    m.insert("reports.export", "Export Report");
    m.insert("reports.print", "Print Report");

    // ==================== MESSAGES ====================
    m.insert("messages.title", "Messages");
    m.insert("messages.compose", "Compose Message");
    m.insert("messages.inbox", "Inbox");
    m.insert("messages.sent", "Sent");
    m.insert("messages.no_messages", "No messages");
    m.insert("messages.send", "Send");
    m.insert("messages.reply", "Reply");

    // ==================== NOTIFICATIONS ====================
    m.insert("notifications.mark_all_read", "Mark all read");
    m.insert("notifications.no_new", "No new notifications");
    m.insert("notifications.view_history", "View All History");

    // ==================== ROLES ====================
    m.insert("roles.school_manager", "School Manager");
    m.insert("roles.teacher", "Teacher");
    m.insert("roles.student", "Student");
    m.insert("roles.parent", "Parent");
    m.insert("roles.admin", "Administrator");

    // ==================== ERRORS ====================
    m.insert(
        "errors.network",
        "Network error. Please check your connection.",
    );
    m.insert("errors.server", "Server error. Please try again later.");
    m.insert("errors.not_found", "The requested resource was not found.");
    m.insert(
        "errors.permission_denied",
        "You don't have permission to perform this action.",
    );
    m.insert(
        "errors.validation",
        "Please check your input and try again.",
    );
    m.insert("errors.unknown", "An unexpected error occurred.");

    // ==================== VALIDATION ====================
    m.insert("validation.required", "This field is required");
    m.insert(
        "validation.email_invalid",
        "Please enter a valid email address",
    );
    m.insert("validation.min_length", "Must be at least {0} characters");
    m.insert("validation.max_length", "Must be at most {0} characters");
    m.insert("validation.grade_range", "Grade must be between 0 and {0}");

    // ==================== ACCESS & ERROR MESSAGES ====================
    m.insert("errors.access_denied", "Access Denied");
    m.insert(
        "errors.access_denied_desc",
        "You don't have permission to access this page.",
    );
    m.insert("errors.go_to_dashboard", "Go to Dashboard");
    m.insert("errors.try_again", "Try Again");
    m.insert("errors.retry_connection", "Retry Connection");
    m.insert("errors.something_wrong", "Something went wrong");
    m.insert("errors.route_access_denied", "Route Access Denied");
    m.insert("errors.required_permission", "Required permission: {0}");
    m.insert("common.vs_last_month", "vs last month");

    m
}

/// Create Farsi translations
fn create_fa_translations() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();

    // ==================== COMMON ====================
    m.insert("common.loading", "در حال بارگذاری...");
    m.insert("common.save", "ذخیره");
    m.insert("common.cancel", "انصراف");
    m.insert("common.delete", "حذف");
    m.insert("common.edit", "ویرایش");
    m.insert("common.create", "ایجاد");
    m.insert("common.search", "جستجو");
    m.insert("common.filter", "فیلتر");
    m.insert("common.view", "مشاهده");
    m.insert("common.close", "بستن");
    m.insert("common.submit", "ارسال");
    m.insert("common.back", "بازگشت");
    m.insert("common.next", "بعدی");
    m.insert("common.confirm", "تأیید");
    m.insert("common.yes", "بله");
    m.insert("common.no", "خیر");
    m.insert("common.success", "موفق");
    m.insert("common.error", "خطا");
    m.insert("common.warning", "هشدار");
    m.insert("common.info", "اطلاعات");
    m.insert("common.required", "اجباری");
    m.insert("common.optional", "اختیاری");
    m.insert("common.actions", "عملیات");
    m.insert("common.status", "وضعیت");
    m.insert("common.date", "تاریخ");
    m.insert("common.time", "زمان");
    m.insert("common.time.ago", "پیش");
    m.insert("common.time.minute", "دقیقه");
    m.insert("common.time.hour", "ساعت");
    m.insert("common.name", "نام");
    m.insert("common.email", "ایمیل");
    m.insert("common.phone", "تلفن");
    m.insert("common.address", "آدرس");
    m.insert("common.description", "توضیحات");
    m.insert("common.details", "جزئیات");
    m.insert("common.settings", "تنظیمات");
    m.insert("common.profile", "پروفایل");
    m.insert("common.logout", "خروج");
    m.insert("common.language", "زبان");
    m.insert("common.select_language", "انتخاب زبان");
    m.insert("common.no_data", "اطلاعاتی موجود نیست");
    m.insert("common.view_details", "مشاهده جزئیات");
    m.insert("common.view_all", "مشاهده همه");
    m.insert("common.portal", "پورتال");
    m.insert("common.open", "باز کردن");
    m.insert("common.subject", "موضوع");
    m.insert("common.term", "ترم");
    m.insert("common.teacher", "معلم");
    m.insert("common.students", "دانش‌آموزان");
    m.insert("common.list", "لیست");
    m.insert("common.grid", "کارت");
    m.insert("common.select_subject", "انتخاب موضوع...");
    m.insert("common.loading_subjects", "در حال بارگذاری موضوعات...");
    m.insert("common.retry", "تلاش مجدد");
    m.insert("common.previous", "قبلی");

    // ==================== TIME FORMATTING ====================
    m.insert("time.just_now", "همین الان");
    m.insert("time.minutes_ago", "{0} دقیقه پیش");
    m.insert("time.hours_ago", "{0} ساعت پیش");
    m.insert("time.days_ago", "{0} روز پیش");

    // ==================== AUTH ====================
    m.insert("auth.sign_in", "ورود");
    m.insert("auth.sign_out", "خروج");
    m.insert("auth.sign_up", "ثبت‌نام");
    m.insert("auth.email", "آدرس ایمیل");
    m.insert("auth.password", "رمز عبور");
    m.insert("auth.confirm_password", "تأیید رمز عبور");
    m.insert("auth.forgot_password", "رمز عبور را فراموش کرده‌اید؟");
    m.insert("auth.reset_password", "بازنشانی رمز عبور");
    m.insert("auth.remember_me", "مرا به خاطر بسپار");
    m.insert("auth.no_account", "حساب کاربری ندارید؟");
    m.insert("auth.have_account", "حساب کاربری دارید؟");
    m.insert("auth.signing_in", "در حال ورود...");
    m.insert("auth.invalid_credentials", "ایمیل یا رمز عبور نامعتبر است");
    m.insert("auth.account_inactive", "حساب کاربری شما غیرفعال شده است");
    m.insert("auth.account_locked", "حساب کاربری شما قفل شده است");
    m.insert("auth.email_not_confirmed", "لطفاً ایمیل خود را تأیید کنید");
    m.insert("auth.invalid_email", "لطفاً یک آدرس ایمیل معتبر وارد کنید");
    m.insert(
        "auth.password_too_short",
        "رمز عبور باید حداقل ۸ کاراکتر باشد",
    );
    m.insert(
        "auth.session_expired",
        "جلسه شما منقضی شده است، لطفاً دوباره وارد شوید",
    );
    m.insert(
        "auth.unauthorized",
        "شما اجازه دسترسی به این منبع را ندارید",
    );
    m.insert("auth.welcome_back", "خوش آمدید");
    m.insert("auth.login_subtitle", "اطلاعات ورود خود را وارد کنید");
    m.insert("auth.protected_by", "محافظت شده توسط امنیت EduTalent");

    // ==================== NAVIGATION ====================
    m.insert("nav.dashboard", "داشبورد");
    m.insert("nav.classes", "کلاس‌ها");
    m.insert("nav.students", "دانش‌آموزان");
    m.insert("nav.teachers", "معلمان");
    m.insert("nav.parents", "اولیا");
    m.insert("nav.assignments", "تکالیف");
    m.insert("nav.grades", "نمرات");
    m.insert("nav.schedule", "برنامه");
    m.insert("nav.reports", "گزارش‌ها");
    m.insert("nav.messages", "پیام‌ها");
    m.insert("nav.notifications", "اعلان‌ها");
    m.insert("nav.settings", "تنظیمات");
    m.insert("nav.help", "راهنما");
    m.insert("nav.materials", "منابع");
    m.insert("nav.submissions", "ارسال‌ها");
    m.insert("nav.attendance", "حضور و غیاب");
    m.insert("nav.users", "کاربران");
    m.insert("nav.requests", "درخواست‌ها");
    m.insert("nav.children", "فرزندان من");
    m.insert("nav.communication", "ارتباطات");
    m.insert("nav.overview", "نمای کلی");
    m.insert("nav.user_management", "مدیریت کاربران");
    m.insert("nav.class_management", "مدیریت کلاس‌ها");
    m.insert("nav.my_classes", "کلاس‌های من");
    m.insert("nav.grading", "نمره‌دهی");
    m.insert("nav.progress", "پیشرفت");
    m.insert("nav.profile", "پروفایل");

    // ==================== MATERIALS ====================
    m.insert("materials.loading", "در حال بارگذاری منابع...");
    m.insert("materials.no_materials_title", "منبعی وجود ندارد");
    m.insert(
        "materials.no_materials_desc",
        "معلم شما هنوز منبعی برای این کلاس بارگذاری نکرده است.",
    );
    m.insert("materials.added_prefix", "تاریخ افزودن: ");
    m.insert("materials.failed_load", "خطا در بارگذاری منابع");
    m.insert("materials.add_new", "افزودن منبع جدید");
    m.insert("materials.title", "عنوان");
    m.insert("materials.title_placeholder", "مثلاً: جزوه فصل ۵");
    m.insert("materials.title_required", "عنوان الزامی است");
    m.insert("materials.description", "توضیحات");
    m.insert(
        "materials.description_placeholder",
        "توضیحات مختصر درباره این منبع...",
    );
    m.insert("materials.file_url", "آدرس فایل");
    m.insert("materials.upload_file", "آپلود فایل");
    m.insert("materials.select_file", "انتخاب فایل");
    m.insert("materials.uploading", "در حال آپلود...");
    m.insert("materials.upload_success", "فایل با موفقیت آپلود شد");
    m.insert("materials.upload_failed", "خطا در آپلود فایل");
    m.insert("materials.or_upload_file", "یا آپلود فایل");
    m.insert("materials.click_to_upload", "برای انتخاب فایل کلیک کنید");
    m.insert("materials.upload_hint", "پشتیبانی از PDF، TXT، MD، HTML");

    // ==================== PARENT DASHBOARD ====================
    m.insert("parent.dashboard.sections.overview", "نمای کلی خانواده");
    m.insert(
        "parent.dashboard.sections.children_progress",
        "پیشرفت فرزندان",
    );
    m.insert("parent.dashboard.sections.quick_actions", "دسترسی سریع");
    m.insert("parent.dashboard.sections.coming_soon", "به زودی");

    m.insert("parent.dashboard.stats.children", "فرزندان");
    m.insert("parent.dashboard.stats.avg_gpa", "میانگین نمرات");
    m.insert("parent.dashboard.stats.messages", "پیام‌ها");
    m.insert("parent.dashboard.stats.events", "رویدادها");
    m.insert("parent.dashboard.stats.status.enrolled", "ثبت‌نام شده");
    m.insert(
        "parent.dashboard.stats.status.family_avg",
        "میانگین خانواده",
    );
    m.insert("parent.dashboard.stats.status.unread", "خوانده نشده");
    m.insert("parent.dashboard.stats.status.upcoming", "پیش‌رو");

    m.insert(
        "parent.dashboard.empty.no_children",
        "هیچ فرزندی به حساب شما متصل نیست",
    );
    m.insert(
        "parent.dashboard.empty.contact_admin",
        "برای اتصال فرزندان خود با مدیریت مدرسه تماس بگیرید.",
    );

    m.insert("parent.dashboard.actions.view_reports", "مشاهده کارنامه‌ها");
    m.insert(
        "parent.dashboard.actions.view_reports_desc",
        "مشاهده گزارش‌های تحصیلی فرزندان",
    );
    m.insert("parent.dashboard.actions.view_classes", "مشاهده کلاس‌ها");
    m.insert(
        "parent.dashboard.actions.view_classes_desc",
        "مشاهده کلاس‌های ثبت‌نام شده و برنامه هفتگی",
    );
    m.insert("parent.dashboard.actions.contact_teacher", "تماس با معلم");
    m.insert(
        "parent.dashboard.actions.contact_teacher_desc",
        "ارسال پیام به معلم فرزندتان",
    );

    m.insert("parent.dashboard.coming_soon.chat", "گفتگوی اولیا و مربیان");
    m.insert(
        "parent.dashboard.coming_soon.chat_desc",
        "پیام‌رسانی مستقیم با معلمان",
    );
    m.insert("parent.dashboard.coming_soon.calendar", "تقویم مدرسه");
    m.insert(
        "parent.dashboard.coming_soon.calendar_desc",
        "مشاهده رویدادهای پیش‌رو",
    );
    m.insert("parent.dashboard.coming_soon.notifications", "اعلان‌ها");
    m.insert(
        "parent.dashboard.coming_soon.notifications_desc",
        "دریافت هشدارها برای بروزرسانی‌های مهم",
    );

    m.insert("parent.dashboard.child_card.gpa", "معدل");
    m.insert("parent.dashboard.child_card.classes", "کلاس‌ها");
    m.insert("parent.dashboard.child_card.view_profile", "مشاهده پروفایل");
    m.insert("parent.dashboard.common.coming_soon_badge", "به زودی");

    // ==================== DASHBOARD ====================
    m.insert("dashboard.welcome", "خوش آمدید");
    m.insert("dashboard.overview", "نمای کلی");
    m.insert("dashboard.quick_actions", "دسترسی سریع");
    m.insert("dashboard.recent_activity", "فعالیت‌های اخیر");
    m.insert("dashboard.upcoming", "پیش رو");
    m.insert("dashboard.statistics", "آمار");
    m.insert("dashboard.total_students", "کل دانش‌آموزان");
    m.insert("dashboard.total_teachers", "کل معلمان");
    m.insert("dashboard.total_classes", "کل کلاس‌ها");
    m.insert("dashboard.pending_grading", "نمره‌دهی در انتظار");
    m.insert("dashboard.active_assignments", "تکالیف فعال");
    m.insert("dashboard.pending_submissions", "ارسال‌های در انتظار");
    m.insert("dashboard.today_schedule", "برنامه امروز");
    m.insert("dashboard.my_progress", "پیشرفت من");
    m.insert("dashboard.enrolled_classes", "کلاس‌های ثبت‌نام شده");
    m.insert("dashboard.pending_tasks", "تکالیف در انتظار");
    m.insert("dashboard.current_gpa", "معدل فعلی");
    m.insert("dashboard.attendance", "حضور و غیاب");
    m.insert("dashboard.upcoming_assignments", "تکالیف پیش رو");
    m.insert("dashboard.my_courses", "دروس من");

    // ==================== GRADES ====================
    m.insert("grades.title", "نمرات");
    m.insert(
        "grades.description",
        "نمرات و پیشرفت تحصیلی خود را بررسی کنید",
    );
    m.insert("grades.gpa", "معدل");
    m.insert("grades.cumulative_gpa", "معدل کل");
    m.insert("grades.current_gpa", "معدل جاری");
    m.insert("grades.credits_completed", "واحدهای گذرانده");
    m.insert("grades.attendance_rate", "نرخ حضور");
    m.insert("grades.by_class", "نمرات بر اساس کلاس");
    m.insert("grades.grade_trends", "روند نمرات");
    m.insert("grades.view_trends", "مشاهده جزئیات روند");
    m.insert("grades.performance_analysis", "تحلیل عملکرد");
    m.insert(
        "grades.track_progress",
        "پیشرفت تحصیلی خود را با تحلیل‌های دقیق دنبال کنید",
    );
    m.insert("grades.grade_details", "جزئیات نمره");
    m.insert("grades.no_classes", "کلاسی یافت نشد");
    m.insert("grades.no_grades", "هنوز تکلیف نمره‌گذاری شده‌ای وجود ندارد");
    m.insert("grades.loading", "در حال بارگذاری نمرات...");
    m.insert("grades.failed_load", "خطا در بارگذاری");
    m.insert("grades.graded_at", "تاریخ نمره‌گذاری");
    m.insert("grades.points", "امتیاز");
    m.insert("grades.academic_trends", "تحلیل روند تحصیلی");
    m.insert("grades.gpa_change", "تغییر معدل این ترم");
    m.insert("grades.avg_score", "میانگین نمره تکالیف");
    m.insert("grades.consistent_improvement", "پیشرفت مداوم");
    m.insert(
        "grades.improvement_desc",
        "نمرات شما در ۳ ماه اخیر بهبود یافته است",
    );
    m.insert("grades.on_time_submissions", "ارسال به موقع");
    m.insert("grades.on_time_desc", "از تکالیف به موقع ارسال شده‌اند");
    m.insert("grades.strong_subject", "درس قوی");
    m.insert("grades.strong_subject_desc", "بهترین عملکرد در");
    m.insert("grades.coming_soon", "نمودارها و تحلیل‌های شخصی به زودی!");
    m.insert("grades.current_performance", "عملکرد تحصیلی فعلی");
    m.insert("grades.scale_100", "از ۱۰۰");
    m.insert("grades.scale_20", "از ۲۰");
    m.insert("grades.total_graded", "تعداد نمره‌دهی شده");
    m.insert("grades.graded_prefix", "تاریخ نمره: ");

    // ==================== CLASSES ====================
    m.insert("classes.title", "کلاس‌ها");
    m.insert("classes.my_classes", "کلاس‌های من");
    m.insert("classes.all_classes", "همه کلاس‌ها");
    m.insert("classes.create_class", "ایجاد کلاس");
    m.insert("classes.class_name", "نام کلاس");
    m.insert("classes.subject", "درس");
    m.insert("classes.teacher", "معلم");
    m.insert("classes.students", "دانش‌آموزان");
    m.insert("classes.schedule", "برنامه");
    m.insert("classes.term", "ترم");
    m.insert("classes.no_classes", "کلاسی یافت نشد");
    m.insert("classes.failed_load", "خطا در بارگذاری کلاس‌ها");
    m.insert("classes.enrolled", "ثبت‌نام شده");
    m.insert("classes.progress", "پیشرفت");
    m.insert("classes.tasks", "وظایف");
    m.insert("classes.materials", "منابع");
    m.insert("classes.with_teacher_prefix", "با ");

    // ==================== ASSIGNMENTS ====================
    m.insert("assignments.title", "تکالیف");
    m.insert("assignments.create", "ایجاد تکلیف");
    m.insert("assignments.due_date", "مهلت ارسال");
    m.insert("assignments.submitted", "ارسال شده");
    m.insert("assignments.pending", "در انتظار");
    m.insert("assignments.overdue", "عقب افتاده");
    m.insert("assignments.completed", "تکمیل شده");
    m.insert("assignments.grading", "در حال نمره‌گذاری");
    m.insert("assignments.submit_work", "ارسال کار");
    m.insert("assignments.view_submission", "مشاهده ارسال");
    m.insert("assignments.no_assignments", "تکلیفی یافت نشد");
    m.insert("assignments.instruction", "دستورالعمل");
    m.insert("assignments.attachments", "پیوست‌ها");
    m.insert("assignments.your_work", "کار شما");
    m.insert("assignments.feedback", "بازخورد");
    m.insert("assignments.loading", "در حال بارگذاری تکالیف...");
    m.insert(
        "assignments.no_class_assignments",
        "تکلیفی برای این کلاس وجود ندارد",
    );
    m.insert("assignments.due_prefix", "مهلت: ");
    m.insert("assignments.description", "مشاهده و ارسال تکالیف شما");
    m.insert("assignments.filter.all", "همه تکالیف");
    m.insert("assignments.loading_failed", "خطا در بارگذاری تکالیف");
    m.insert("assignments.empty.all", "هنوز تکلیفی وجود ندارد");
    m.insert("assignments.empty.filtered", "تکلیف {0} یافت نشد");
    m.insert(
        "assignments.empty.check_back",
        "بعداً برای تکالیف جدید مراجعه کنید",
    );
    m.insert("assignments.action.start", "شروع تکلیف");
    m.insert("assignments.action.view_feedback", "مشاهده بازخورد");
    m.insert("assignments.action.save_draft", "ذخیره پیش‌نویس و بستن");
    m.insert("assignments.action.submit", "ارسال تکلیف");
    m.insert("assignments.action.submitting", "در حال ارسال...");
    m.insert("assignments.points", " امتیاز");
    m.insert("assignments.status_prefix", "وضعیت: ");
    m.insert(
        "assignments.personalization.info",
        "این تکلیف بر اساس استعدادها و سبک یادگیری منحصر به فرد شما شخصی‌سازی شده است.",
    );
    m.insert("assignments.personalization.badge", "شخصی‌سازی شده برای شما");
    m.insert(
        "assignments.personalization.details_title",
        "جزئیات شخصی‌سازی",
    );
    m.insert("assignments.personalization.details", "جزئیات شخصی‌سازی");
    m.insert("assignments.personalization.difficulty", "دشواری: ");
    m.insert("assignments.personalization.est_time", "زمان تخمینی: ");
    m.insert("assignments.work.title", "انجام تکلیف");
    m.insert(
        "assignments.work.placeholder",
        "پاسخ تکلیف خود را اینجا بنویسید...",
    );
    m.insert("assignments.work.characters", " کاراکتر");
    m.insert(
        "assignments.work.empty_error",
        "لطفاً قبل از ارسال چیزی بنویسید",
    );
    m.insert("assignments.work.submit_error", "خطا در ارسال");
    m.insert("assignments.details.not_found", "تکلیف یافت نشد");
    m.insert(
        "assignments.ai_personalizing.title",
        "هوش مصنوعی در حال سفارشی‌سازی تکلیف شماست",
    );
    m.insert(
        "assignments.ai_personalizing.description",
        "لطفاً صبر کنید تا هوش مصنوعی این تکلیف را برای استعدادها و سبک یادگیری شما تنظیم کند...",
    );

    // ==================== SUBMISSIONS ====================
    m.insert("submissions.title", "ارسال‌ها");
    m.insert("submissions.grade_submission", "نمره‌گذاری ارسال");
    m.insert("submissions.enter_grade", "نمره را وارد کنید");
    m.insert("submissions.feedback_optional", "بازخورد (اختیاری)");
    m.insert("submissions.submit_grade", "ثبت نمره");
    m.insert("submissions.graded", "نمره‌گذاری شده");
    m.insert("submissions.not_graded", "نمره‌گذاری نشده");
    m.insert("submissions.submitted_at", "زمان ارسال");
    m.insert("submissions.no_submissions", "ارسالی یافت نشد");
    m.insert("submissions.grade_label", "Grade");
    m.insert(
        "submissions.review_description",
        "Review and grade student submissions",
    );
    m.insert("submissions.pending_filter", "Pending");
    m.insert("submissions.all_filter", "All");
    m.insert("submissions.failed_load", "Failed to load submissions: ");
    m.insert("submissions.caught_up_title", "All Caught Up!");
    m.insert(
        "submissions.caught_up_desc",
        "No pending submissions to grade",
    );
    m.insert("submissions.update_grade", "Update Grade");
    m.insert("submissions.grade_btn", "Grade Submission");
    m.insert("submissions.grade_modal_title", "Grade Submission");
    m.insert(
        "submissions.validation_range",
        "Please enter a valid grade between 0 and 100",
    );
    m.insert(
        "submissions.save_failed",
        "Failed to save grade. Please try again.",
    );
    m.insert("submissions.student_work_label", "Student's Work");
    m.insert("submissions.grade_range_label", "Grade (0-100)");
    m.insert(
        "submissions.feedback_placeholder",
        "Great work! Consider improving...",
    );
    m.insert("submissions.saving_btn", "Saving...");
    m.insert("submissions.save_btn", "Save Grade");

    // ==================== TEACHER STUDENTS ====================
    m.insert("teacher_students.title", "دانش‌آموزان");
    m.insert("teacher_students.all_students", "همه دانش‌آموزان");
    m.insert("teacher_students.student_name", "نام دانش‌آموز");
    m.insert("teacher_students.grade_level", "پایه تحصیلی");
    m.insert("teacher_students.enrolled_classes", "کلاس‌های ثبت‌نام شده");
    m.insert("teacher_students.view_profile", "مشاهده پروفایل");
    m.insert("teacher_students.view_grades", "مشاهده نمرات");
    m.insert("teacher_students.no_students", "دانش‌آموزی یافت نشد");

    // ==================== STUDENTS ====================
    m.insert("students.title", "دانش‌آموزان");
    m.insert("students.all_students", "همه دانش‌آموزان");
    m.insert("students.student_name", "نام دانش‌آموز");
    m.insert("students.grade_level", "پایه تحصیلی");
    m.insert("students.enrolled_classes", "کلاس‌های ثبت‌نام شده");
    m.insert("students.view_profile", "مشاهده پروفایل");
    m.insert("students.view_grades", "مشاهده نمرات");
    m.insert("students.no_students", "دانش‌آموزی یافت نشد");
    m.insert("students.loading", "در حال بارگذاری دانش‌آموزان...");
    m.insert("students.failed_load", "خطا در بارگذاری دانش‌آموزان");
    m.insert(
        "students.no_enrolled_class",
        "دانش‌آموزی در این کلاس ثبت‌نام نکرده است",
    );
    m.insert("students.total", "تعداد کل دانش‌آموزان");
    m.insert("students.submitted_count", "ارسال شده: ");
    m.insert("students.graded_count", "نمره داده شده: ");

    // ==================== TEACHERS ====================
    m.insert("students.title", "دانش‌آموزان");
    m.insert("students.all_students", "همه دانش‌آموزان");
    m.insert("students.student_name", "نام دانش‌آموز");
    m.insert("students.grade_level", "پایه تحصیلی");
    m.insert("students.enrolled_classes", "کلاس‌های ثبت‌نام شده");
    m.insert("students.view_profile", "مشاهده پروفایل");
    m.insert("students.view_grades", "مشاهده نمرات");
    m.insert("students.no_students", "دانش‌آموزی یافت نشد");

    // ==================== TEACHERS ====================
    m.insert("teachers.title", "معلمان");
    m.insert("teachers.all_teachers", "همه معلمان");
    m.insert("teachers.department", "گروه آموزشی");
    m.insert("teachers.assigned_classes", "کلاس‌های تدریس");
    m.insert("teachers.no_teachers", "معلمی یافت نشد");
    m.insert(
        "teachers.dashboard.no_assignments_created",
        "هنوز تکلیفی ایجاد نشده است",
    );
    m.insert(
        "teachers.dashboard.create_first_assignment",
        "اولین تکلیف خود را ایجاد کنید",
    );
    m.insert(
        "teachers.dashboard.no_classes_assigned",
        "کلاسی اختصاص داده نشده",
    );
    m.insert("teachers.dashboard.course_progress", "پیشرفت دوره");
    m.insert("teachers.status.active", "فعال");
    m.insert("teachers.status.enrolled", "ثبت‌نام شده");
    m.insert("teachers.status.to_review", "جهت بررسی");
    m.insert(
        "teachers.quick_actions.create_assignment_desc",
        "تکلیف جدید برای کلاس شما",
    );
    m.insert(
        "teachers.quick_actions.grade_submissions",
        "نمره‌دهی ارسال‌ها",
    );
    m.insert(
        "teachers.quick_actions.grade_submissions_desc",
        "بررسی و نمره‌دهی کارها",
    );
    m.insert("teachers.quick_actions.schedule_lecture", "زمان‌بندی کلاس");
    m.insert(
        "teachers.quick_actions.schedule_lecture_desc",
        "برنامه‌ریزی جلسه بعدی",
    );
    m.insert(
        "teachers.classes.manage_description",
        "مدیریت کلاس‌ها و پیگیری پیشرفت دانش‌آموزان",
    );
    m.insert("teachers.classes.no_classes_yet", "هنوز کلاسی وجود ندارد");
    m.insert(
        "teachers.classes.no_classes_assigned_desc",
        "شما هنوز به هیچ کلاسی اختصاص داده نشده‌اید.",
    );
    m.insert("teachers.classes.enrolled_suffix", " دانش‌آموز ثبت‌نام شده");
    m.insert("teachers.classes.actions.grading", "نمره‌دهی");
    m.insert("teachers.classes.modal.overview_suffix", " - نمای کلی");
    m.insert(
        "teachers.classes.enrolled_students_label",
        "دانش‌آموزان ثبت‌نام شده",
    );
    m.insert("teachers.classes.modal.students_suffix", " - دانش‌آموزان");
    m.insert("teachers.classes.modal.grading_suffix", " - نمره‌دهی");
    m.insert("teachers.classes.assignments.status.draft", "پیش‌نویس");
    m.insert(
        "teachers.classes.assignments.to_grade_suffix",
        " برای نمره‌دهی",
    );
    m.insert(
        "teachers.classes.assignments.total_assigned",
        "کل اختصاص داده شده: ",
    );
    m.insert(
        "teachers.assignments.manage_description",
        "ایجاد و مدیریت تکالیف برای کلاس‌های شما",
    );
    m.insert(
        "teachers.assignments.delete_success",
        "تکلیف با موفقیت حذف شد",
    );
    m.insert("teachers.assignments.delete_failed", "خطا در حذف: ");
    m.insert("teachers.assignments.create_new", "ایجاد تکلیف جدید");
    m.insert(
        "teachers.assignments.no_assignments_title",
        "هنوز تکلیفی وجود ندارد",
    );
    m.insert(
        "teachers.assignments.no_assignments_desc",
        "اولین تکلیف خود را ایجاد کنید",
    );
    m.insert("teachers.assignments.submission_progress", "پیشرفت ارسال‌ها");
    m.insert("teachers.assignments.delete_tooltip", "حذف تکلیف");
    m.insert(
        "teachers.assignments.create.success",
        "تکلیف با موفقیت ایجاد شد!",
    );
    m.insert("teachers.assignments.create.failed", "خطا در ایجاد: ");
    m.insert("teachers.assignments.create.title_label", "* عنوان تکلیف");
    m.insert(
        "teachers.assignments.create.title_placeholder",
        "مثلاً: آزمون فصل ۵",
    );
    m.insert("teachers.assignments.create.class_label", "* کلاس");
    m.insert(
        "teachers.assignments.create.select_class",
        "یک کلاس انتخاب کنید...",
    );
    m.insert(
        "teachers.assignments.create.loading_classes",
        "در حال بارگذاری کلاس‌ها...",
    );
    m.insert("teachers.assignments.create.due_date_label", "* مهلت تحویل");
    m.insert(
        "teachers.assignments.create.description_label",
        "* توضیحات تکلیف",
    );
    m.insert(
        "teachers.assignments.create.description_placeholder",
        "الزامات تکلیف را شرح دهید...",
    );
    m.insert(
        "teachers.assignments.create.materials_label",
        "منابع مرجع (برای زمینه هوش مصنوعی)",
    );
    m.insert(
        "teachers.assignments.create.materials_selected",
        "منبع انتخاب شده",
    );
    m.insert(
        "teachers.assignments.create.ai_title",
        "شخصی‌سازی با هوش مصنوعی",
    );
    m.insert("teachers.assignments.create.ai_desc", "پس از انتشار، می‌توانید این تکلیف را برای هر دانش‌آموز با استفاده از هوش مصنوعی شخصی‌سازی کنید. سیستم تکلیف را بر اساس استعدادها و سبک یادگیری هر دانش‌آموز سفارشی می‌کند.");
    m.insert(
        "teachers.assignments.create.creating_btn",
        "در حال ایجاد...",
    );
    m.insert("teachers.assignments.create.create_btn", "ایجاد تکلیف");
    m.insert("teachers.assignments.details.title", "جزئیات تکلیف");
    m.insert(
        "teachers.assignments.publish.success",
        "تکلیف با موفقیت منتشر شد!",
    );
    m.insert("teachers.assignments.publish.failed", "خطا در انتشار: ");
    m.insert("teachers.assignments.details.not_found", "تکلیف یافت نشد");
    m.insert(
        "teachers.assignments.details.failed_load",
        "خطا در بارگذاری تکلیف",
    );
    m.insert("teachers.assignments.details.created_label", "ایجاد شده");
    m.insert("teachers.assignments.details.publish_btn", "انتشار تکلیف");
    m.insert(
        "teachers.assignments.validation.required_fields",
        "لطفاً تمام فیلدهای اجباری را پر کنید",
    );
    m.insert(
        "teachers.assignments.validation.invalid_date",
        "فرمت تاریخ نامعتبر است",
    );
    m.insert(
        "teachers.students.description",
        "مشاهده پروفایل دانش‌آموزان و پیگیری پیشرفت آنها",
    );
    m.insert(
        "teachers.students.search_placeholder",
        "جستجوی دانش‌آموزان...",
    );
    m.insert("teachers.students.no_students_found", "دانش‌آموزی یافت نشد");
    m.insert(
        "teachers.students.no_students_desc",
        "شما هنوز دانش‌آموزی در کلاس‌های خود ندارید.",
    );
    m.insert("teachers.students.submitted_label", "ارسال شده");
    m.insert("teachers.students.profile_btn", "پروفایل");
    m.insert("teachers.students.grades_btn", "نمرات");
    m.insert("teachers.students.profile_title_suffix", " - پروفایل");
    m.insert("teachers.students.average_label", "معدل");
    m.insert("teachers.students.submitted_stat", "ارسال شده");
    m.insert("teachers.students.classes_stat", "کلاس‌ها");
    m.insert("teachers.students.enrolled_classes", "کلاس‌های ثبت‌نام شده");
    m.insert("teachers.students.grades_title_suffix", " - نمرات");
    m.insert(
        "teachers.students.loading_grades",
        "در حال بارگذاری نمرات...",
    );
    m.insert("teachers.students.grades_failed", "خطا در بارگذاری: ");
    m.insert(
        "teachers.students.no_grades",
        "هنوز تکلیف نمره‌گذاری شده‌ای وجود ندارد",
    );
    m.insert("teachers.students.average_grade", "میانگین نمرات");

    // ==================== TEACHER SUBMISSIONS ====================
    m.insert(
        "submissions.review_description",
        "بررسی و نمره‌دهی ارسال‌های دانش‌آموزان",
    );
    m.insert("submissions.pending_filter", "در انتظار");
    m.insert("submissions.all_filter", "همه");
    m.insert("submissions.failed_load", "خطا در بارگذاری ارسال‌ها: ");
    m.insert("submissions.caught_up_title", "همه کارها انجام شد!");
    m.insert(
        "submissions.caught_up_desc",
        "تکلیف نمره‌گذاری نشده‌ای وجود ندارد",
    );
    m.insert("submissions.update_grade", "بروزرسانی نمره");
    m.insert("submissions.grade_btn", "نمره‌دهی ارسال");
    m.insert("submissions.grade_modal_title", "نمره‌دهی ارسال");
    m.insert(
        "submissions.validation_range",
        "لطفاً نمره‌ای معتبر بین ۰ تا ۱۰۰ وارد کنید",
    );
    m.insert(
        "submissions.save_failed",
        "خطا در ذخیره نمره. لطفاً دوباره تلاش کنید.",
    );
    m.insert("submissions.student_work_label", "کار دانش‌آموز");
    m.insert("submissions.grade_range_label", "نمره (۰-۱۰۰)");
    m.insert(
        "submissions.feedback_placeholder",
        "کار عالی بود! برای بهبود در نظر بگیرید...",
    );
    m.insert("submissions.saving_btn", "در حال ذخیره...");
    m.insert("submissions.save_btn", "ذخیره نمره");

    // ==================== SCHEDULE ====================
    m.insert("schedule.title", "برنامه");
    m.insert("schedule.description", "مشاهده برنامه کلاسی و تاریخ‌های مهم");
    m.insert("schedule.today", "برنامه امروز");
    m.insert("schedule.weekly_overview", "نمای هفتگی");
    m.insert("schedule.important_dates", "تاریخ‌های مهم");
    m.insert("schedule.classes_today", "کلاس امروز");
    m.insert("schedule.classes_count", "کلاس");
    m.insert("schedule.instructor_prefix", "مدرس: ");
    m.insert("schedule.status.in_progress", "در حال برگزاری");
    m.insert("schedule.status.completed", "پایان یافته");
    m.insert("schedule.status.upcoming", "پیش رو");

    // ==================== PARENTS ====================
    m.insert("parents.title", "اولیا");
    m.insert("parents.my_children", "فرزندان من");
    m.insert("parents.child_progress", "پیشرفت فرزند");
    m.insert("parents.view_report", "مشاهده گزارش");
    m.insert("parents.contact_teacher", "تماس با معلم");
    m.insert("parents.no_children", "فرزندی ثبت نشده است");

    // ==================== SETTINGS ====================
    m.insert("settings.title", "تنظیمات");
    m.insert("settings.general", "تنظیمات عمومی");
    m.insert("settings.notifications", "تنظیمات اعلان‌ها");
    m.insert("settings.security", "تنظیمات امنیت");
    m.insert("settings.profile", "تنظیمات پروفایل");
    m.insert("settings.timezone", "منطقه زمانی");
    m.insert("settings.date_format", "فرمت تاریخ");
    m.insert("settings.time_format", "فرمت زمان");
    m.insert("settings.save_changes", "ذخیره تغییرات");
    m.insert("settings.changes_saved", "تغییرات با موفقیت ذخیره شد");

    // ==================== REPORTS ====================
    m.insert("reports.title", "گزارش‌ها");
    m.insert("reports.generate", "ایجاد گزارش");
    m.insert("reports.class_performance", "عملکرد کلاس");
    m.insert("reports.student_progress", "پیشرفت دانش‌آموز");
    m.insert("reports.attendance_report", "گزارش حضور و غیاب");
    m.insert("reports.grade_distribution", "توزیع نمرات");
    m.insert("reports.export", "خروجی گزارش");
    m.insert("reports.print", "چاپ گزارش");

    // ==================== MESSAGES ====================
    m.insert("messages.title", "پیام‌ها");
    m.insert("messages.compose", "نوشتن پیام");
    m.insert("messages.inbox", "صندوق ورودی");
    m.insert("messages.sent", "ارسال شده");
    m.insert("messages.no_messages", "پیامی وجود ندارد");
    m.insert("messages.send", "ارسال");
    m.insert("messages.reply", "پاسخ");

    // ==================== NOTIFICATIONS ====================
    m.insert("notifications.mark_all_read", "خواندن همه");
    m.insert("notifications.no_new", "اعلان جدیدی وجود ندارد");
    m.insert("notifications.view_history", "مشاهده همه");

    // ==================== ROLES ====================
    m.insert("roles.school_manager", "مدیر مدرسه");
    m.insert("roles.teacher", "معلم");
    m.insert("roles.student", "دانش‌آموز");
    m.insert("roles.parent", "والدین");
    m.insert("roles.admin", "مدیر سیستم");

    // ==================== ERRORS ====================
    m.insert("errors.network", "خطای شبکه. لطفاً اتصال خود را بررسی کنید.");
    m.insert("errors.server", "خطای سرور. لطفاً بعداً تلاش کنید.");
    m.insert("errors.not_found", "منبع درخواستی یافت نشد.");
    m.insert(
        "errors.permission_denied",
        "شما اجازه انجام این عملیات را ندارید.",
    );
    m.insert(
        "errors.validation",
        "لطفاً ورودی خود را بررسی کرده و دوباره تلاش کنید.",
    );
    m.insert("errors.unknown", "خطای غیرمنتظره‌ای رخ داد.");

    // ==================== VALIDATION ====================
    m.insert("validation.required", "این فیلد اجباری است");
    m.insert(
        "validation.email_invalid",
        "لطفاً یک آدرس ایمیل معتبر وارد کنید",
    );
    m.insert("validation.min_length", "باید حداقل {0} کاراکتر باشد");
    m.insert("validation.max_length", "باید حداکثر {0} کاراکتر باشد");
    m.insert(
        "school_manager.users.export_modal.export_btn",
        "خروجی گرفتن",
    );

    // ==================== ACCESS & ERROR MESSAGES ====================
    m.insert("errors.access_denied", "دسترسی غیرمجاز");
    m.insert(
        "errors.access_denied_desc",
        "شما اجازه دسترسی به این صفحه را ندارید.",
    );
    m.insert("errors.go_to_dashboard", "برو به داشبورد");
    m.insert("errors.try_again", "تلاش مجدد");
    m.insert("errors.retry_connection", "تلاش مجدد برای اتصال");
    m.insert("errors.something_wrong", "مشکلی پیش آمد");
    m.insert("errors.route_access_denied", "دسترسی به مسیر مسدود است");
    m.insert("errors.required_permission", "دسترسی مورد نیاز: {0}");
    m.insert("common.vs_last_month", "نسبت به ماه گذشته");

    // ==================== CLASS MANAGEMENT ====================
    m.insert("school_manager.classes.title", "مدیریت کلاس‌ها");
    m.insert(
        "school_manager.classes.description",
        "مدیریت کلاس‌ها، دوره‌ها و برنامه‌های تحصیلی",
    );
    m.insert("school_manager.classes.active_classes", "کلاس‌های فعال");
    m.insert("school_manager.classes.actions.new_class", "کلاس جدید");

    m.insert("school_manager.classes.empty.title", "هنوز کلاسی وجود ندارد");
    m.insert(
        "school_manager.classes.empty.desc",
        "اولین کلاس خود را ایجاد کنید تا مدیریت دوره‌ها و دانش‌آموزان را شروع کنید.",
    );
    m.insert(
        "school_manager.classes.error.load_failed",
        "خطا در بارگذاری کلاس‌ها",
    );

    m.insert(
        "school_manager.classes.create_modal.title",
        "ایجاد کلاس جدید",
    );
    m.insert("school_manager.classes.create_modal.class_name", "نام کلاس");
    m.insert(
        "school_manager.classes.create_modal.class_name_placeholder",
        "مثال: ریاضی ۱۰۱ - بخش الف",
    );
    m.insert(
        "school_manager.classes.create_modal.term_placeholder",
        "مثال: پاییز ۱۴۰۳",
    );
    m.insert(
        "school_manager.classes.create_modal.create_btn",
        "ایجاد کلاس",
    );
    m.insert(
        "school_manager.classes.create_modal.creating",
        "در حال ایجاد...",
    );

    m.insert(
        "school_manager.classes.detail_modal.title",
        "{class} - مدیریت دانش‌آموزان",
    );
    m.insert(
        "school_manager.classes.detail_modal.add_student",
        "افزودن دانش‌آموز",
    );
    m.insert(
        "school_manager.classes.detail_modal.select_student",
        "انتخاب دانش‌آموز...",
    );
    m.insert(
        "school_manager.classes.detail_modal.error_loading_students",
        "خطا در بارگذاری دانش‌آموزان",
    );
    m.insert(
        "school_manager.classes.detail_modal.enrolled_students",
        "دانش‌آموزان ثبت‌نام شده",
    );
    m.insert(
        "school_manager.classes.detail_modal.no_students",
        "هنوز دانش‌آموزی ثبت‌نام نشده است",
    );
    m.insert(
        "school_manager.classes.detail_modal.failed_load_students",
        "خطا در بارگذاری دانش‌آموزان",
    );
    m.insert("school_manager.classes.detail_modal.enroll_btn", "ثبت‌نام");

    m.insert(
        "school_manager.classes.errors.name_required",
        "نام کلاس الزامی است",
    );
    m.insert(
        "school_manager.classes.errors.subject_required",
        "لطفاً یک موضوع را انتخاب کنید",
    );
    m.insert(
        "school_manager.classes.errors.term_required",
        "ترم الزامی است",
    );
    m.insert(
        "school_manager.classes.errors.select_student_required",
        "لطفاً یک دانش‌آموز را انتخاب کنید",
    );
    m.insert(
        "school_manager.classes.errors.enroll_failed",
        "خطا در ثبت‌نام: ",
    );
    m.insert(
        "school_manager.classes.errors.remove_failed",
        "خطا در حذف: ",
    );

    // ==================== REPORTS ====================
    m.insert("school_manager.reports.title", "گزارش‌ها و تحلیل‌ها");
    m.insert(
        "school_manager.reports.description",
        "بینش جامع و گزارش‌های مربوط به موسسه شما",
    );
    m.insert("school_manager.reports.config.title", "پیکربندی گزارش");
    m.insert("school_manager.reports.config.export", "📥 خروجی");
    m.insert("school_manager.reports.config.generate", "📊 تولید گزارش");

    m.insert(
        "school_manager.reports.types.class_performance",
        "عملکرد کلاس",
    );
    m.insert(
        "school_manager.reports.types.class_performance_desc",
        "نمرات دانش‌آموزان و آمار کلاس",
    );
    m.insert(
        "school_manager.reports.types.teacher_workload",
        "بار کاری معلم",
    );
    m.insert(
        "school_manager.reports.types.teacher_workload_desc",
        "تکالیف و ساعات کاری",
    );
    m.insert(
        "school_manager.reports.types.attendance",
        "حضور و غیاب دانش‌آموزان",
    );
    m.insert(
        "school_manager.reports.types.attendance_desc",
        "سوابق حضور و غیاب",
    );
    m.insert(
        "school_manager.reports.types.parent_engagement",
        "تعامل والدین",
    );
    m.insert(
        "school_manager.reports.types.parent_engagement_desc",
        "معیارهای فعالیت پورتال",
    );

    m.insert("school_manager.reports.filters.class_label", "کلاس/موضوع");
    m.insert("school_manager.reports.filters.all_classes", "همه کلاس‌ها");
    m.insert("school_manager.reports.filters.teacher_label", "معلم");
    m.insert("school_manager.reports.filters.all_teachers", "همه معلمان");
    m.insert(
        "school_manager.reports.filters.student_label",
        "دانش‌آموز (اختیاری)",
    );
    m.insert(
        "school_manager.reports.filters.all_students",
        "همه دانش‌آموزان",
    );
    m.insert(
        "school_manager.reports.filters.date_range_label",
        "بازه زمانی",
    );

    m.insert(
        "school_manager.reports.filters.ranges.this_week",
        "این هفته",
    );
    m.insert(
        "school_manager.reports.filters.ranges.this_month",
        "این ماه",
    );
    m.insert(
        "school_manager.reports.filters.ranges.this_semester",
        "این ترم",
    );
    m.insert("school_manager.reports.filters.ranges.this_year", "امسال");
    m.insert("school_manager.reports.filters.ranges.custom", "سفارشی");

    m.insert(
        "school_manager.reports.class_performance.title",
        "گزارش عملکرد کلاس",
    );
    m.insert(
        "school_manager.reports.class_performance.subtitle_all",
        "همه کلاس‌ها • {date}",
    );
    m.insert(
        "school_manager.reports.class_performance.subtitle_filtered",
        "{filter} • {date}",
    );
    m.insert(
        "school_manager.reports.class_performance.export_pdf",
        "📥 PDF",
    );
    m.insert(
        "school_manager.reports.class_performance.export_excel",
        "📊 اکسل",
    );

    m.insert(
        "school_manager.reports.stats.total_reports",
        "مجموع گزارش‌ها",
    );
    m.insert("school_manager.reports.stats.available", "موجود");
    m.insert("school_manager.reports.stats.no_data", "داده‌ای نیست");
    m.insert("school_manager.reports.stats.students", "دانش‌آموزان");
    m.insert("school_manager.reports.stats.tracked", "پیگیری شده");
    m.insert("school_manager.reports.stats.teachers", "معلمان");
    m.insert("school_manager.reports.stats.active", "فعال");
    m.insert("school_manager.reports.stats.date_range", "بازه زمانی");
    m.insert("school_manager.reports.stats.selected", "انتخاب شده");

    m.insert("school_manager.reports.chart.title", "تجسم نمودار عملکرد");
    m.insert(
        "school_manager.reports.chart.desc",
        "توزیع نمرات و تحلیل روند (فاز بعدی)",
    );

    m.insert("school_manager.reports.table.title", "جزئیات گزارش‌ها");
    m.insert("school_manager.reports.table.student", "دانش‌آموز");
    m.insert("school_manager.reports.table.teacher", "معلم");
    m.insert("school_manager.reports.table.email", "ایمیل دانش‌آموز");
    m.insert("school_manager.reports.table.summary", "خلاصه هوش مصنوعی");
    m.insert("school_manager.reports.table.created", "ایجاد شده");
    m.insert(
        "school_manager.reports.table.empty",
        "گزارشی برای فیلترهای انتخاب شده یافت نشد",
    );
    m.insert(
        "school_manager.reports.table.loading",
        "در حال بارگذاری گزارش‌ها...",
    );
    m.insert("school_manager.reports.table.unknown_student", "ناشناس");
    m.insert(
        "school_manager.reports.table.unassigned_teacher",
        "تعیین نشده",
    );
    m.insert("school_manager.reports.table.no_summary", "بدون خلاصه");

    m.insert("school_manager.reports.sidebar.summary_title", "خلاصه گزارش");
    m.insert("school_manager.reports.sidebar.type_label", "نوع گزارش:");
    m.insert("school_manager.reports.sidebar.period_label", "دوره:");
    m.insert(
        "school_manager.reports.sidebar.generated_label",
        "تولید شده:",
    );
    m.insert("school_manager.reports.sidebar.just_now", "همین الان");

    m.insert(
        "school_manager.reports.sidebar.export_title",
        "گزینه‌های خروجی",
    );
    m.insert("school_manager.reports.sidebar.export_pdf", "📄 خروجی PDF");
    m.insert(
        "school_manager.reports.sidebar.export_excel",
        "📊 خروجی اکسل",
    );
    m.insert("school_manager.reports.sidebar.export_csv", "📑 خروجی CSV");
    m.insert(
        "school_manager.reports.sidebar.export_image",
        "🖼️ خروجی تصویر",
    );

    m.insert(
        "school_manager.reports.sidebar.schedule_title",
        "زمان‌بندی گزارش‌ها",
    );
    m.insert(
        "school_manager.reports.sidebar.schedule_weekly",
        "⏰ زمان‌بندی هفتگی",
    );
    m.insert(
        "school_manager.reports.sidebar.schedule_monthly",
        "📅 زمان‌بندی ماهانه",
    );
    m.insert(
        "school_manager.reports.sidebar.schedule_quarterly",
        "📆 زمان‌بندی فصلی",
    );

    m.insert(
        "school_manager.reports.workload.title",
        "گزارش بار کاری معلم",
    );
    m.insert(
        "school_manager.reports.workload.analysis",
        "تحلیل بار کاری معلم",
    );
    m.insert(
        "school_manager.reports.workload.desc",
        "ساعات تدریس، تکالیف کلاس و توزیع بار کاری (فاز بعدی)",
    );

    m.insert(
        "school_manager.reports.attendance.title",
        "گزارش حضور و غیاب دانش‌آموز",
    );
    m.insert(
        "school_manager.reports.attendance.analytics",
        "تحلیل حضور و غیاب",
    );
    m.insert(
        "school_manager.reports.attendance.desc",
        "الگوهای حضور و غیاب، روندها و گزارش‌های استثنا (فاز بعدی)",
    );

    m.insert(
        "school_manager.reports.engagement.title",
        "گزارش تعامل والدین",
    );
    m.insert(
        "school_manager.reports.engagement.analytics",
        "تحلیل پورتال والدین",
    );
    m.insert(
        "school_manager.reports.engagement.desc",
        "فرکانس ورود، استفاده از امکانات و معیارهای تعامل (فاز بعدی)",
    );

    // Dashboard Activity
    m.insert(
        "school_manager.activity.new_student_added",
        "دانش‌آموز جدید \"{0}\" اضافه شد.",
    );

    // Requests
    m.insert(
        "school_manager.requests.title",
        "درخواست‌های تغییر پروفایل معلق",
    );
    m.insert(
        "school_manager.requests.no_auth_token",
        "توکن احراز هویت یافت نشد",
    );
    m.insert(
        "school_manager.requests.success",
        "درخواست با موفقیت {0} شد",
    );
    m.insert(
        "school_manager.requests.failure",
        "خطا در تصمیم‌گیری برای درخواست: {0}",
    );
    m.insert(
        "school_manager.requests.empty",
        "هیچ درخواست معلقی وجود ندارد",
    );
    m.insert(
        "school_manager.requests.requested_by",
        "درخواست شده توسط: {0}",
    );
    m.insert("school_manager.requests.reject", "رد کردن");
    m.insert("school_manager.requests.approve", "تایید کردن");
    m.insert(
        "school_manager.requests.error",
        "خطا در بارگذاری درخواست‌ها: {0}",
    );
    m.insert(
        "school_manager.requests.loading",
        "در حال بارگذاری درخواست‌ها...",
    );

    // System Health
    m.insert("school_manager.system_health", "سلامت سیستم");
    m.insert("school_manager.health.database", "پایگاه داده");
    m.insert("school_manager.health.api_latency", "تاخیر API");
    m.insert("school_manager.health.storage", "فضای ذخیره‌سازی");
    m.insert("school_manager.health.active_users", "کاربران فعال");
    m.insert("school_manager.health.status.healthy", "سالم");
    m.insert("school_manager.health.status.good", "خوب");
    m.insert("school_manager.health.status.moderate", "متوسط");
    m.insert("school_manager.health.status.normal", "عادی");

    // Quick Actions
    m.insert("school_manager.quick_actions.title", "دسترسی سریع");
    m.insert("school_manager.actions.add_user", "افزودن کاربر");
    m.insert(
        "school_manager.actions.add_user_desc",
        "ایجاد حساب کاربری جدید برای دانش‌آموز، معلم یا والدین.",
    );
    m.insert("school_manager.actions.create_class", "ایجاد کلاس");
    m.insert(
        "school_manager.actions.create_class_desc",
        "افزودن کلاس جدید و تخصیص معلم.",
    );
    m.insert("school_manager.actions.view_reports", "مشاهده گزارش‌ها");
    m.insert(
        "school_manager.actions.view_reports_desc",
        "تولید و مشاهده گزارش‌های سیستم.",
    );
    m.insert("school_manager.actions.system_settings", "تنظیمات سیستم");
    m.insert(
        "school_manager.actions.system_settings_desc",
        "پیکربندی ترجیحات سیستم.",
    );

    // Recent Activity
    m.insert("school_manager.recent_activity", "فعالیت‌های اخیر");
    m.insert(
        "school_manager.recent_activity_desc",
        "نمای کلی از آخرین به‌روزرسانی‌های سیستم",
    );
    m.insert(
        "school_manager.activity.new_student_class_added",
        "دانش‌آموز جدید \"{0}\" به {1} اضافه شد.",
    );
    m.insert(
        "school_manager.activity.schedule_updated",
        "برنامه کلاسی \"{0}\" به‌روز شد.",
    );
    m.insert(
        "school_manager.activity.report_generated",
        "گزارش نهایی {0} تولید شد.",
    );

    // Parent Children Section
    m.insert("parent.children.title", "فرزندان من");
    m.insert(
        "parent.children.desc",
        "اطلاعات دقیق درباره پیشرفت تحصیلی فرزندان شما",
    );
    m.insert(
        "parent.children.error",
        "خطا در بارگذاری اطلاعات فرزندان: {0}",
    );
    m.insert("parent.children.empty.title", "هیچ فرزندی متصل نیست");
    m.insert(
        "parent.children.empty.desc",
        "برای اتصال فرزندان خود به حساب کاربری، با مدیریت مدرسه تماس بگیرید.",
    );

    m.insert("parent.children.actions.view_grades", "مشاهده نمرات");
    m.insert("parent.children.actions.attendance", "حضور و غیاب");
    m.insert("parent.children.actions.message_teacher", "پیام به معلم");
    m.insert("parent.children.actions.assignments", "تکالیف");

    m.insert("parent.children.grades.current_gpa", "معدل فعلی");
    m.insert("parent.children.grades.loading", "در حال بارگذاری نمرات...");
    m.insert("parent.children.grades.failed", "خطا در بارگذاری: {0}");
    m.insert("parent.children.grades.empty", "هنوز نمره‌ای ثبت نشده است");

    m.insert(
        "parent.children.attendance.loading",
        "در حال بارگذاری حضور و غیاب...",
    );
    m.insert("parent.children.attendance.failed", "خطا در بارگذاری: {0}");
    m.insert("parent.children.attendance.present", "حاضر");
    m.insert("parent.children.attendance.absent", "غایب");
    m.insert("parent.children.attendance.rate", "نرخ حضور");
    m.insert("parent.children.attendance.recent_absences", "غیبت‌های اخیر");

    m.insert(
        "parent.children.assignments.loading",
        "در حال بارگذاری تکالیف...",
    );
    m.insert("parent.children.assignments.failed", "خطا در بارگذاری: {0}");
    m.insert("parent.children.assignments.empty", "تکلیفی یافت نشد");
    m.insert("parent.children.assignments.due", "مهلت: {0}");

    m.insert("parent.children.assignments.empty", "تکلیفی یافت نشد");
    m.insert("parent.children.assignments.due", "مهلت: {0}");

    // Parent Communication Section
    m.insert("parent.communication.title", "ارتباطات");
    m.insert(
        "parent.communication.desc",
        "پیام‌ها و بروزرسانی‌ها از طرف معلمان و مدیریت مدرسه",
    );
    m.insert("parent.communication.compose.title", "ارسال پیام جدید");
    m.insert("parent.communication.compose.to", "به:");
    m.insert("parent.communication.compose.child", "فرزند:");
    m.insert("parent.communication.compose.subject", "موضوع:");
    m.insert(
        "parent.communication.compose.subject_ph",
        "موضوع را وارد کنید...",
    );
    m.insert("parent.communication.compose.message", "پیام:");
    m.insert(
        "parent.communication.compose.message_ph",
        "پیام خود را اینجا بنویسید...",
    );
    m.insert("parent.communication.compose.send", "ارسال پیام");
    m.insert(
        "parent.communication.compose.options.all_teachers",
        "همه معلمان",
    );

    m.insert("parent.communication.messages.title", "پیام‌های اخیر");
    m.insert("parent.communication.messages.re", "عطف به: {0}");
    m.insert("parent.communication.messages.reply", "پاسخ");
    m.insert("parent.communication.messages.archive", "بایگانی");

    m.insert("parent.communication.messages.reply", "پاسخ");
    m.insert("parent.communication.messages.archive", "بایگانی");

    // Parent Reports Section
    m.insert("parent.reports.title", "گزارش‌ها و آمار");
    m.insert(
        "parent.reports.desc",
        "مشاهده گزارش‌ها و تحلیل‌های دقیق برای فرزندان شما",
    );
    m.insert("parent.reports.filters.title", "ایجاد گزارش سفارشی");
    m.insert("parent.reports.filters.child", "فرزند:");
    m.insert("parent.reports.filters.type", "نوع گزارش:");
    m.insert("parent.reports.filters.period", "بازه زمانی:");
    m.insert("parent.reports.filters.generate", "ایجاد گزارش");

    m.insert("parent.reports.filters.options.all_children", "همه فرزندان");
    m.insert("parent.reports.filters.options.academic", "عملکرد تحصیلی");
    m.insert(
        "parent.reports.filters.options.attendance",
        "گزارش حضور و غیاب",
    );
    m.insert("parent.reports.filters.options.behavior", "گزارش انضباطی");
    m.insert("parent.reports.filters.options.comprehensive", "گزارش جامع");
    m.insert(
        "parent.reports.filters.options.current_semester",
        "نیم‌سال جاری",
    );
    m.insert("parent.reports.filters.options.last_month", "ماه گذشته");
    m.insert(
        "parent.reports.filters.options.last_quarter",
        "سه ماهه گذشته",
    );
    m.insert("parent.reports.filters.options.academic_year", "سال تحصیلی");

    m.insert("parent.reports.available.title", "گزارش‌های موجود");
    m.insert("parent.reports.available.academic.title", "عملکرد تحصیلی");
    m.insert(
        "parent.reports.available.academic.desc",
        "بررسی جامع تحصیلی شامل نمرات، روند معدل و عملکرد در دروس",
    );
    m.insert(
        "parent.reports.available.attendance.title",
        "گزارش حضور و غیاب",
    );
    m.insert(
        "parent.reports.available.attendance.desc",
        "سوابق دقیق حضور و غیاب شامل غیبت‌ها، تاخیرها و الگوها",
    );
    m.insert("parent.reports.available.behavior.title", "گزارش انضباطی");
    m.insert(
        "parent.reports.available.behavior.desc",
        "ارزیابی‌های انضباطی، سوابق رفتاری و بازخورد معلمان",
    );
    m.insert(
        "parent.reports.available.standardized.title",
        "آزمون‌های استاندارد",
    );
    m.insert(
        "parent.reports.available.standardized.desc",
        "نمرات آزمون‌های استاندارد و پیگیری پیشرفت",
    );

    m.insert("parent.reports.available.updated", "بروزرسانی: {0}");
    m.insert("parent.reports.available.for", "برای: {0}");
    m.insert("parent.reports.available.download", "دانلود PDF");

    m.insert("parent.reports.recent.title", "دانلودهای اخیر");

    // Settings
    m.insert("school_manager.settings.title", "تنظیمات و پروفایل");
    m.insert(
        "school_manager.settings.description",
        "مدیریت تنظیمات حساب و موسسه",
    );
    m.insert("school_manager.settings.tabs.profile", "پروفایل");
    m.insert("school_manager.settings.tabs.security", "امنیت");
    m.insert("school_manager.settings.tabs.general", "عمومی");
    m.insert("school_manager.settings.tabs.notifications", "اعلانات");

    // General Settings
    m.insert("school_manager.settings.general.title", "تنظیمات عمومی");
    m.insert(
        "school_manager.settings.general.loading",
        "در حال بارگذاری ترجیحات...",
    );
    m.insert("school_manager.settings.general.timezone", "منطقه زمانی");
    m.insert("school_manager.settings.general.language", "زبان");
    m.insert("school_manager.settings.general.date_format", "فرمت تاریخ");
    m.insert("school_manager.settings.general.time_format", "فرمت زمان");
    m.insert("school_manager.settings.general.save_btn", "ذخیره تنظیمات");
    m.insert(
        "school_manager.settings.general.success",
        "موفقیت! تنظیمات ذخیره شد.",
    );
    m.insert("school_manager.settings.general.error", "خطا: {0}");

    // Timezone Options
    m.insert(
        "school_manager.settings.general.timezone.utc",
        "هماهنگ جهانی (UTC)",
    );
    m.insert(
        "school_manager.settings.general.timezone.et",
        "وقت شرقی (ET)",
    );
    m.insert(
        "school_manager.settings.general.timezone.ct",
        "وقت مرکزی (CT)",
    );
    m.insert(
        "school_manager.settings.general.timezone.mt",
        "وقت کوهستانی (MT)",
    );
    m.insert(
        "school_manager.settings.general.timezone.pt",
        "وقت اقیانوس آرام (PT)",
    );
    m.insert("school_manager.settings.general.timezone.gmt", "لندن (GMT)");
    m.insert(
        "school_manager.settings.general.timezone.cet",
        "پاریس (CET)",
    );
    m.insert(
        "school_manager.settings.general.timezone.jst",
        "توکیو (JST)",
    );
    m.insert("school_manager.settings.general.timezone.gst", "دبی (GST)");
    m.insert(
        "school_manager.settings.general.timezone.aedt",
        "سیدنی (AEDT)",
    );

    // Time Format Options
    m.insert(
        "school_manager.settings.general.time_format.24h",
        "۲۴ ساعته (۱۴:۳۰)",
    );
    m.insert(
        "school_manager.settings.general.time_format.12h",
        "۱۲ ساعته (۲:۳۰ ب.ظ)",
    );

    // Notification Settings
    m.insert(
        "school_manager.settings.notifications.title",
        "ترجیحات اعلان‌ها",
    );
    m.insert(
        "school_manager.settings.notifications.loading",
        "در حال بارگذاری ترجیحات...",
    );
    m.insert(
        "school_manager.settings.notifications.channels",
        "کانال‌های اعلان",
    );
    m.insert("school_manager.settings.notifications.types", "انواع اعلان");
    m.insert("school_manager.settings.notifications.digest", "خلاصه ایمیل");
    m.insert(
        "school_manager.settings.notifications.save_btn",
        "ذخیره ترجیحات",
    );
    m.insert(
        "school_manager.settings.notifications.success",
        "موفقیت! ترجیحات ذخیره شد.",
    );
    m.insert("school_manager.settings.notifications.error", "خطا: {0}");

    // Profile Settings
    m.insert("school_manager.settings.profile.info_title", "اطلاعات شخصی");
    m.insert(
        "school_manager.settings.profile.loading",
        "در حال بارگذاری پروفایل...",
    );
    m.insert("school_manager.settings.profile.full_name", "نام کامل");
    m.insert("school_manager.settings.profile.email", "آدرس ایمیل");
    m.insert("school_manager.settings.profile.phone", "شماره تلفن");
    m.insert("school_manager.settings.profile.office", "محل دفتر");
    m.insert("school_manager.settings.profile.hours", "ساعات کاری");
    m.insert("school_manager.settings.profile.emergency", "تماس اضطراری");
    m.insert("school_manager.settings.profile.save_btn", "ذخیره تغییرات");
    m.insert(
        "school_manager.settings.profile.updated",
        "پروفایل بروزرسانی شد",
    );
    m.insert("school_manager.settings.profile.role_admin", "مدیر سیستم");
    m.insert(
        "school_manager.settings.profile.actions_title",
        "عملیات پروفایل",
    );
    m.insert(
        "school_manager.settings.profile.request_change",
        "درخواست تغییر پروفایل",
    );
    m.insert(
        "school_manager.settings.profile.change_pwd",
        "تغییر رمز عبور",
    );
    m.insert(
        "school_manager.settings.profile.pwd_requirements",
        "الزامات رمز عبور:",
    );
    m.insert(
        "school_manager.settings.profile.pwd_req_1",
        "حداقل ۸ کاراکتر",
    );
    m.insert(
        "school_manager.settings.profile.pwd_req_2",
        "حداقل یک حرف بزرگ",
    );
    m.insert("school_manager.settings.profile.pwd_req_3", "حداقل یک عدد");
    m.insert(
        "school_manager.settings.profile.pwd_coming_soon",
        "قابلیت تغییر رمز عبور به زودی. در حال حاضر از طریق ارائه دهنده احراز هویت مدیریت می‌شود.",
    );
    m.insert(
        "school_manager.settings.profile.request_submitted",
        "درخواست ارسال شد",
    );
    m.insert(
        "school_manager.settings.profile.log.updated",
        "پروفایل بروزرسانی شد",
    );
    m.insert(
        "school_manager.settings.profile.log.submitted",
        "درخواست ثبت شد",
    );

    m.insert(
        "school_manager.settings.notifications.email",
        "اعلان‌های ایمیلی",
    );
    m.insert("school_manager.settings.notifications.push", "اعلان‌های پوش");
    m.insert(
        "school_manager.settings.notifications.in_app",
        "اعلان‌های درون‌برنامه",
    );

    m.insert(
        "school_manager.settings.notifications.user_reg",
        "ثبت‌نام کاربر",
    );
    m.insert(
        "school_manager.settings.notifications.user_reg_desc",
        "اعلان هنگام پیوستن کاربر جدید به سیستم",
    );
    m.insert(
        "school_manager.settings.notifications.class_created",
        "ایجاد کلاس",
    );
    m.insert(
        "school_manager.settings.notifications.class_created_desc",
        "اعلان هنگام ایجاد کلاس جدید",
    );
    m.insert(
        "school_manager.settings.notifications.assignment",
        "ارسال تکلیف",
    );
    m.insert(
        "school_manager.settings.notifications.assignment_desc",
        "اعلان هنگام ارسال تکلیف توسط دانش‌آموزان",
    );
    m.insert(
        "school_manager.settings.notifications.report",
        "تولید گزارش",
    );
    m.insert(
        "school_manager.settings.notifications.report_desc",
        "اعلان هنگام تولید گزارش دانش‌آموز",
    );
    m.insert(
        "school_manager.settings.notifications.profile_change",
        "درخواست تغییر پروفایل",
    );
    m.insert(
        "school_manager.settings.notifications.profile_change_desc",
        "اعلان هنگام ارسال درخواست تغییر پروفایل",
    );
    m.insert(
        "school_manager.settings.notifications.announcements",
        "اعلانات سیستم",
    );
    m.insert(
        "school_manager.settings.notifications.announcements_desc",
        "دریافت بروزرسانی‌های مهم سیستم",
    );

    m.insert("school_manager.settings.notifications.digest.never", "هرگز");
    m.insert(
        "school_manager.settings.notifications.digest.daily",
        "خلاصه روزانه",
    );
    m.insert(
        "school_manager.settings.notifications.digest.weekly",
        "خلاصه هفتگی",
    );

    // Security Settings
    m.insert("school_manager.settings.security.title", "تغییر رمز عبور");
    m.insert(
        "school_manager.settings.security.current_pwd",
        "رمز عبور فعلی",
    );
    m.insert("school_manager.settings.security.new_pwd", "رمز عبور جدید");
    m.insert(
        "school_manager.settings.security.confirm_pwd",
        "تأیید رمز عبور جدید",
    );
    m.insert(
        "school_manager.settings.security.update_btn",
        "بروزرسانی رمز عبور",
    );
    m.insert(
        "school_manager.settings.security.mismatch",
        "رمزهای عبور جدید مطابقت ندارند",
    );
    m.insert(
        "school_manager.settings.security.min_length",
        "رمز عبور باید حداقل ۸ کاراکتر باشد",
    );
    m.insert(
        "school_manager.settings.security.success",
        "رمز عبور با موفقیت تغییر کرد",
    );
    m.insert(
        "school_manager.settings.security.failure",
        "خطا در تغییر رمز عبور",
    );

    // User Creation Hub
    m.insert("school_manager.users.creation.title", "مرکز ایجاد کاربر");
    m.insert(
        "school_manager.users.creation.subtitle",
        "ایجاد و مدیریت حساب‌های دانش‌آموز، معلم و والدین",
    );
    m.insert("school_manager.users.creation.cancel", "لغو");
    m.insert("school_manager.users.creation.import", "📥 وارد کردن گروهی");
    m.insert("school_manager.users.creation.tabs.students", "دانش‌آموزان");
    m.insert("school_manager.users.creation.tabs.teachers", "معلمان");
    m.insert("school_manager.users.creation.tabs.parents", "والدین");

    m.insert("school_manager.users.creation.personal_info", "اطلاعات شخصی");
    m.insert(
        "school_manager.users.creation.academic_info",
        "اطلاعات تحصیلی",
    );
    m.insert(
        "school_manager.users.creation.professional_info",
        "اطلاعات شغلی",
    );
    m.insert(
        "school_manager.users.creation.class_assignment",
        "تخصیص کلاس",
    );
    m.insert(
        "school_manager.users.creation.student_association",
        "ارتباط با دانش‌آموز",
    );
    m.insert("school_manager.users.creation.creating", "در حال ایجاد...");

    m.insert("school_manager.users.creation.first_name", "نام *");
    m.insert("school_manager.users.creation.last_name", "نام خانوادگی *");
    m.insert("school_manager.users.creation.full_name", "نام کامل *");
    m.insert("school_manager.users.creation.email", "آدرس ایمیل *");
    m.insert("school_manager.users.creation.phone", "شماره تلفن");
    m.insert("school_manager.users.creation.dob", "تاریخ تولد *");
    m.insert(
        "school_manager.users.creation.student_id",
        "شناسه دانش‌آموزی *",
    );
    m.insert("school_manager.users.creation.grade_level", "پایه تحصیلی *");
    m.insert(
        "school_manager.users.creation.enrollment_date",
        "تاریخ ثبت‌نام *",
    );
    m.insert("school_manager.users.creation.class_section", "کلاس/بخش");
    m.insert(
        "school_manager.users.creation.academic_year",
        "سال تحصیلی *",
    );
    m.insert(
        "school_manager.users.creation.employee_id",
        "شناسه کارمندی *",
    );
    m.insert("school_manager.users.creation.department", "دپارتمان *");
    m.insert("school_manager.users.creation.subjects", "دروس *");
    m.insert("school_manager.users.creation.hire_date", "تاریخ استخدام *");
    m.insert(
        "school_manager.users.creation.qualifications",
        "مدارک و گواهینامه‌ها",
    );
    m.insert(
        "school_manager.users.creation.assign_classes",
        "تخصیص کلاس‌ها",
    );
    m.insert("school_manager.users.creation.parent_id", "شناسه والدین *");
    m.insert("school_manager.users.creation.relationship", "نسبت *");
    m.insert(
        "school_manager.users.creation.associated_students",
        "دانش‌آموزان مرتبط *",
    );

    // Dropdown Options
    m.insert("school_manager.users.creation.grades.9", "پایه نهم");
    m.insert("school_manager.users.creation.grades.10", "پایه دهم");
    m.insert("school_manager.users.creation.grades.11", "پایه یازدهم");
    m.insert("school_manager.users.creation.grades.12", "پایه دوازدهم");

    m.insert("school_manager.users.creation.sections.a", "بخش الف");
    m.insert("school_manager.users.creation.sections.b", "بخش ب");
    m.insert("school_manager.users.creation.sections.c", "بخش ج");

    m.insert("school_manager.users.creation.subjects.math", "ریاضیات");
    m.insert("school_manager.users.creation.subjects.physics", "فیزیک");
    m.insert("school_manager.users.creation.subjects.chemistry", "شیمی");
    m.insert(
        "school_manager.users.creation.subjects.biology",
        "زیست‌شناسی",
    );
    m.insert(
        "school_manager.users.creation.subjects.english",
        "ادبیات انگلیسی",
    );
    m.insert("school_manager.users.creation.subjects.history", "تاریخ");
    m.insert("school_manager.users.creation.subjects.cs", "علوم کامپیوتر");

    m.insert("school_manager.users.creation.class_assignment_help", "کلاس‌های مورد نظر را برای تخصیص به این معلم انتخاب کنید. برای انتخاب چند مورد، کلید Ctrl/Cmd را نگه دارید.");
    m.insert("school_manager.users.creation.student_association_help", "یک یا چند دانش‌آموز مرتبط با این ولی را انتخاب کنید. برای انتخاب چند مورد، کلید Ctrl/Cmd را نگه دارید.");

    m.insert(
        "school_manager.users.creation.placeholders.first_name",
        "نام را وارد کنید",
    );
    m.insert(
        "school_manager.users.creation.placeholders.last_name",
        "نام خانوادگی را وارد کنید",
    );
    m.insert(
        "school_manager.users.creation.placeholders.full_name",
        "نام کامل را وارد کنید",
    );
    m.insert(
        "school_manager.users.creation.placeholders.relationship",
        "نسبت را وارد کنید (مثلاً پدر)",
    );

    // Sidebar Stats & Tips - Students
    m.insert(
        "school_manager.users.creation.stats.student.total",
        "Total Students",
    );
    m.insert(
        "school_manager.users.creation.stats.student.new",
        "New This Week",
    );
    m.insert(
        "school_manager.users.creation.stats.student.pending",
        "Pending Approval",
    );
    m.insert(
        "school_manager.users.creation.stats.student.total_change",
        "+12 this month",
    );
    m.insert(
        "school_manager.users.creation.stats.student.new_change",
        "+3 vs last week",
    );
    m.insert(
        "school_manager.users.creation.stats.student.pending_change",
        "Need review",
    );

    m.insert(
        "school_manager.users.creation.tips.student.id",
        "Student IDs should follow the STU format (STU123456)",
    );
    m.insert(
        "school_manager.users.creation.tips.student.email",
        "Welcome emails are sent automatically to new students",
    );
    m.insert(
        "school_manager.users.creation.tips.student.parent",
        "Parent association is required for student accounts",
    );

    // Sidebar Stats & Tips - Teachers
    m.insert(
        "school_manager.users.creation.stats.teacher.total",
        "Total Teachers",
    );
    m.insert(
        "school_manager.users.creation.stats.teacher.active",
        "Active Classes",
    );
    m.insert(
        "school_manager.users.creation.stats.teacher.avg",
        "Avg Students/Class",
    );
    m.insert(
        "school_manager.users.creation.stats.teacher.total_change",
        "+2 this month",
    );
    m.insert(
        "school_manager.users.creation.stats.teacher.active_change",
        "All assigned",
    );
    m.insert(
        "school_manager.users.creation.stats.teacher.avg_change",
        "Optimal range",
    );

    m.insert(
        "school_manager.users.creation.tips.teacher.subjects",
        "Teachers can teach multiple subjects and grades",
    );
    m.insert(
        "school_manager.users.creation.tips.teacher.cert",
        "Certifications should be current and verifiable",
    );
    m.insert(
        "school_manager.users.creation.tips.teacher.assign",
        "Class assignments are made after account creation",
    );

    // Sidebar Stats & Tips - Parents
    m.insert(
        "school_manager.users.creation.stats.parent.total",
        "Total Parents",
    );
    m.insert(
        "school_manager.users.creation.stats.parent.linked",
        "Linked Students",
    );
    m.insert(
        "school_manager.users.creation.stats.parent.engagement",
        "Engagement Rate",
    );
    m.insert(
        "school_manager.users.creation.stats.parent.total_change",
        "+8 this month",
    );
    m.insert(
        "school_manager.users.creation.stats.parent.linked_change",
        "Some parents have multiple",
    );
    m.insert(
        "school_manager.users.creation.stats.parent.engagement_change",
        "Excellent",
    );

    m.insert(
        "school_manager.users.creation.tips.parent.multiple",
        "Each parent can be linked to multiple students",
    );
    m.insert(
        "school_manager.users.creation.tips.parent.access",
        "Parents have access to their children's academic progress",
    );
    m.insert(
        "school_manager.users.creation.tips.parent.mobile",
        "Parent accounts receive mobile notifications for important updates",
    );

    // Activity
    m.insert(
        "school_manager.users.creation.activity.student.created",
        "John Smith - Student account created",
    );
    m.insert(
        "school_manager.users.creation.activity.teacher.created",
        "Ms. Johnson - Teacher account created",
    );
    m.insert(
        "school_manager.users.creation.activity.parent.created",
        "Mary Davis - Parent account created",
    );
    m.insert(
        "school_manager.users.creation.activity.student.email",
        "Welcome emails sent to 5 new students",
    );
    m.insert(
        "school_manager.users.creation.activity.teacher.updated",
        "Class assignments updated for 3 teachers",
    );
    m.insert(
        "school_manager.users.creation.activity.parent.access",
        "Parent portal access granted to 2 parents",
    );
    m.insert(
        "school_manager.users.creation.activity.time.2h",
        "2 hours ago",
    );
    m.insert(
        "school_manager.users.creation.activity.time.5h",
        "5 hours ago",
    );
    m.insert(
        "school_manager.users.creation.placeholders.email_student",
        "student@school.edu",
    );
    m.insert(
        "school_manager.users.creation.placeholders.email_teacher",
        "teacher@school.edu",
    );
    m.insert(
        "school_manager.users.creation.placeholders.email_parent",
        "parent@example.com",
    );
    m.insert(
        "school_manager.users.creation.placeholders.phone",
        "(555) 123-4567",
    );
    m.insert(
        "school_manager.users.creation.placeholders.student_id",
        "STU001234",
    );
    m.insert(
        "school_manager.users.creation.placeholders.employee_id",
        "TCH001234",
    );
    m.insert(
        "school_manager.users.creation.placeholders.parent_id",
        "PAR001234",
    );
    m.insert(
        "school_manager.users.creation.placeholders.qualifications",
        "لیست مدارک، گواهینامه‌ها و صلاحیت‌ها...",
    );

    m.insert(
        "school_manager.users.creation.btn.create_student",
        "ایجاد حساب دانش‌آموز",
    );
    m.insert(
        "school_manager.users.creation.btn.create_teacher",
        "ایجاد حساب معلم",
    );
    m.insert(
        "school_manager.users.creation.btn.create_parent",
        "ایجاد حساب والدین",
    );

    m.insert(
        "school_manager.users.creation.options.select_grade",
        "انتخاب پایه تحصیلی",
    );
    m.insert(
        "school_manager.users.creation.options.select_section",
        "انتخاب بخش",
    );
    m.insert(
        "school_manager.users.creation.options.select_dept",
        "انتخاب دپارتمان",
    );
    m.insert(
        "school_manager.users.creation.options.loading_classes",
        "در حال بارگذاری کلاس‌ها...",
    );
    m.insert(
        "school_manager.users.creation.options.loading_students",
        "در حال بارگذاری دانش‌آموزان...",
    );

    m.insert(
        "school_manager.users.creation.success.student",
        "دانش‌آموز با موفقیت ایجاد شد! رمز عبور موقت: {0}",
    );
    m.insert(
        "school_manager.users.creation.success.teacher",
        "معلم با موفقیت ایجاد شد! رمز عبور موقت: {0}",
    );
    m.insert(
        "school_manager.users.creation.success.parent",
        "حساب والدین با موفقیت ایجاد شد. رمز عبور موقت: {0}",
    );
    m.insert(
        "school_manager.users.creation.error.parent",
        "خطا در ایجاد والدین: {0}",
    );

    m.insert("school_manager.users.creation.stats.title", "آمار فعلی");
    m.insert("school_manager.users.creation.tips.title", "نکات سریع");
    m.insert(
        "school_manager.users.creation.activity.title",
        "فعالیت‌های اخیر",
    );

    m.insert("validation.grade_range", "نمره باید بین ۰ تا {0} باشد");

    m.insert("validation.grade_range", "نمره باید بین ۰ تا {0} باشد");

    // ==================== SCHOOL MANAGER ====================
    m.insert("school_manager.access_denied", "دسترسی غیرمجاز");
    m.insert(
        "school_manager.access_denied_desc",
        "شما اجازه دسترسی به داشبورد مدیر مدرسه را ندارید.",
    );
    m.insert("school_manager.go_to_dashboard", "برو به داشبورد خودتان");

    m.insert("school_manager.users.title", "مدیریت کاربران");
    m.insert(
        "school_manager.users.description",
        "مدیریت دانش‌آموزان، معلمان و والدین در موسسه شما",
    );
    m.insert("school_manager.users.summary.students", "دانش‌آموزان");
    m.insert("school_manager.users.summary.teachers", "معلمان");
    m.insert("school_manager.users.summary.parents", "والدین");
    m.insert(
        "school_manager.users.manage_btn.students",
        "مدیریت دانش‌آموزان",
    );
    m.insert("school_manager.users.manage_btn.teachers", "مدیریت معلمان");
    m.insert("school_manager.users.manage_btn.parents", "مدیریت والدین");

    m.insert("school_manager.users.tabs.directory", "فهرست کاربران");
    m.insert("school_manager.users.tabs.requests", "درخواست‌های تغییر");

    m.insert("school_manager.users.actions.add_user", "افزودن کاربر");
    m.insert(
        "school_manager.users.actions.bulk_import",
        "وارد کردن گروهی",
    );
    m.insert(
        "school_manager.users.actions.export_users",
        "خروجی گرفتن کاربران",
    );

    m.insert("school_manager.users.directory.title", "فهرست کاربران");
    m.insert(
        "school_manager.users.directory.search_placeholder",
        "جستجوی کاربران...",
    );
    m.insert("school_manager.users.directory.all_roles", "همه نقش‌ها");
    m.insert("school_manager.users.directory.all_status", "همه وضعیت‌ها");
    m.insert("school_manager.users.directory.active", "فعال");
    m.insert("school_manager.users.directory.inactive", "غیرفعال");

    m.insert("school_manager.users.table.name", "نام");
    m.insert("school_manager.users.table.role", "نقش");
    m.insert("school_manager.users.table.status", "وضعیت");
    m.insert("school_manager.users.table.joined", "تاریخ عضویت");
    m.insert("school_manager.users.table.actions", "عملیات");

    m.insert("school_manager.users.actions.edit", "ویرایش");
    m.insert("school_manager.users.actions.deactivate", "غیرفعال کردن");
    m.insert("school_manager.users.actions.reactivate", "فعال کردن مجدد");

    m.insert(
        "school_manager.users.messages.deactivate_success",
        "کاربر با موفقیت غیرفعال شد",
    );
    m.insert(
        "school_manager.users.messages.deactivate_fail",
        "خطا در غیرفعال کردن کاربر: ",
    );
    m.insert(
        "school_manager.users.messages.reactivate_success",
        "کاربر با موفقیت فعال شد",
    );
    m.insert(
        "school_manager.users.messages.reactivate_fail",
        "خطا در فعال کردن کاربر: ",
    );
    m.insert(
        "school_manager.users.messages.update_success",
        "اطلاعات کاربر با موفقیت بروزرسانی شد",
    );
    m.insert(
        "school_manager.users.messages.update_fail",
        "خطا در بروزرسانی کاربر: ",
    );
    m.insert(
        "school_manager.users.messages.load_error",
        "خطا در بارگذاری کاربران: {e}",
    );

    m.insert("school_manager.users.edit_modal.title", "ویرایش کاربر");
    m.insert("school_manager.users.edit_modal.saving", "در حال ذخیره...");
    m.insert("school_manager.users.edit_modal.save", "ذخیره تغییرات");

    m.insert(
        "school_manager.users.import_modal.title",
        "وارد کردن گروهی کاربران",
    );
    m.insert(
        "school_manager.users.import_modal.csv_title",
        "فرمت CSV مورد نیاز",
    );
    m.insert(
        "school_manager.users.import_modal.csv_desc",
        "یک فایل CSV با ستون‌های: name, email, role آپلود کنید",
    );
    m.insert(
        "school_manager.users.import_modal.drop_text",
        "فایل CSV خود را اینجا رها کنید",
    );
    m.insert(
        "school_manager.users.import_modal.browse_text",
        "یا برای انتخاب فایل کلیک کنید",
    );
    m.insert(
        "school_manager.users.import_modal.coming_soon",
        "قابلیت وارد کردن گروهی به زودی فعال می‌شود. لطفاً فعلاً کاربران را به صورت تکی اضافه کنید.",
    );
    m.insert("school_manager.users.import_modal.import_btn", "وارد کردن");

    m.insert(
        "school_manager.users.export_modal.title",
        "خروجی گرفتن کاربران",
    );
    m.insert(
        "school_manager.users.export_modal.format_label",
        "فرمت خروجی",
    );
    m.insert("school_manager.users.export_modal.options_label", "گزینه‌ها");
    m.insert(
        "school_manager.users.export_modal.include_inactive",
        "شامل کاربران غیرفعال",
    );
    m.insert(
        "school_manager.users.export_modal.coming_soon",
        "قابلیت خروجی گرفتن به زودی فعال می‌شود.",
    );
    m.insert(
        "school_manager.users.export_modal.export_btn",
        "خروجی گرفتن",
    );

    m.insert("classes.not_enrolled", "در هیچ کلاسی ثبت‌نام نشده‌اید");

    m.insert(
        "classes.view_description",
        "کلاس‌های ثبت‌نام‌شده و منابع کلاس خود را مشاهده کنید",
    );
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_lookup() {
        assert_eq!(t("auth.sign_in", Locale::En), "Sign In");
        assert_eq!(t("auth.sign_in", Locale::Fa), "ورود");
    }

    #[test]
    fn test_fallback_to_english() {
        // If a key is missing in Farsi, it should fall back to English
        // For this test, we'd need a key that exists only in English
        // Since we have full translations, let's test the unknown key fallback
        assert_eq!(t("unknown.key", Locale::En), "unknown.key");
    }

    #[test]
    fn test_all_fa_keys_present() {
        let en_translations = create_en_translations();
        let fa_translations = create_fa_translations();

        for key in en_translations.keys() {
            assert!(
                fa_translations.contains_key(key),
                "Missing Farsi translation for key: {}",
                key
            );
        }
    }
}
