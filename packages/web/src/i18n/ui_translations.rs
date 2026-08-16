use super::Locale;

/// Small, explicit UI translation table for copy introduced after the original
/// monolithic translation map. Keeping these keys here prevents the UI from
/// exposing raw translation identifiers while the legacy table is gradually
/// decomposed into feature modules.
pub(crate) fn supplemental_translation(
    key: &'static str,
    locale: Locale,
) -> Option<&'static str> {
    match (locale, key) {
        (Locale::En, "common.refresh") => Some("Refresh"),
        (Locale::Fa, "common.refresh") => Some("تازه‌سازی"),

        (Locale::En, "submissions.review_description") => {
            Some("Review and grade student submissions")
        }
        (Locale::En, "submissions.pending_filter") => Some("Pending"),
        (Locale::En, "submissions.all_filter") => Some("All"),
        (Locale::En, "submissions.failed_load") => Some("Failed to load submissions: "),
        (Locale::En, "submissions.caught_up_title") => Some("All Caught Up!"),
        (Locale::En, "submissions.caught_up_desc") => {
            Some("No pending submissions to grade")
        }
        (Locale::En, "submissions.update_grade") => Some("Update Grade"),
        (Locale::En, "submissions.grade_btn") => Some("Grade Submission"),
        (Locale::En, "submissions.grade_modal_title") => Some("Grade Submission"),
        (Locale::En, "submissions.validation_range") => Some("Please enter a valid numeric grade"),
        (Locale::En, "submissions.save_failed") => {
            Some("Failed to save grade. Please try again.")
        }
        (Locale::En, "submissions.student_work_label") => Some("Student's Work"),
        (Locale::En, "submissions.grade_range_label") => Some("Grade (0-100)"),
        (Locale::En, "submissions.feedback_placeholder") => {
            Some("Great work! Consider improving...")
        }
        (Locale::En, "submissions.saving_btn") => Some("Saving..."),
        (Locale::En, "submissions.save_btn") => Some("Save Grade"),

        // Persian uses a 0–20 display scale. Override the legacy 0–100 copy so
        // validation and labels agree with Locale::max_grade().
        (Locale::Fa, "submissions.validation_range") => Some("لطفاً یک نمره عددی معتبر وارد کنید"),
        (Locale::Fa, "submissions.grade_range_label") => Some("نمره (۰-۲۰)"),
        (Locale::Fa, "submissions.grade_label") => Some("نمره"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teacher_submission_copy_is_localized_in_english() {
        for key in [
            "common.refresh",
            "submissions.review_description",
            "submissions.pending_filter",
            "submissions.all_filter",
            "submissions.grade_btn",
            "submissions.grade_modal_title",
            "submissions.save_btn",
        ] {
            assert_ne!(supplemental_translation(key, Locale::En), None, "missing {key}");
        }
    }

    #[test]
    fn persian_grading_copy_uses_the_twenty_point_scale() {
        assert_eq!(
            supplemental_translation("submissions.grade_range_label", Locale::Fa),
            Some("نمره (۰-۲۰)")
        );
    }
}
