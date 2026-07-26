//! Grading utilities for locale-specific grade handling
//! 
//! Implements the approved strategy: Store grades in the user's preferred scale
//! - Farsi (fa): 0-20 scale (Iranian education standard)
//! - English (en): 0-100 scale (International percentage-based)

use super::Locale;

/// Grade value with its associated scale
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalizedGrade {
    /// The grade value in the locale's scale
    pub value: f64,
    /// The locale (determines the scale: 20 for Fa, 100 for En)
    pub locale: Locale,
}

impl LocalizedGrade {
    /// Create a new localized grade
    pub fn new(value: f64, locale: Locale) -> Self {
        Self { value, locale }
    }

    /// Create a grade for Farsi locale (0-20 scale)
    pub fn farsi(value: f64) -> Self {
        Self::new(value, Locale::Fa)
    }

    /// Create a grade for English locale (0-100 scale)
    pub fn english(value: f64) -> Self {
        Self::new(value, Locale::En)
    }

    /// Get the maximum possible grade for this locale
    pub fn max(&self) -> f64 {
        self.locale.max_grade()
    }

    /// Get the grade as a percentage (0-100)
    pub fn as_percentage(&self) -> f64 {
        match self.locale {
            Locale::En => self.value,
            Locale::Fa => (self.value / 20.0) * 100.0,
        }
    }

    /// Convert this grade to another locale's scale
    pub fn convert_to(&self, target_locale: Locale) -> Self {
        if self.locale == target_locale {
            return *self;
        }

        let percentage = self.as_percentage();
        let new_value = match target_locale {
            Locale::En => percentage,
            Locale::Fa => (percentage / 100.0) * 20.0,
        };

        Self::new(new_value, target_locale)
    }

    /// Format the grade for display
    pub fn format_display(&self) -> String {
        match self.locale {
            Locale::En => format!("{:.0}%", self.value),
            Locale::Fa => {
                // Use Persian numerals and format as X از ۲۰
                let value_str = to_persian_numerals(format!("{:.1}", self.value));
                let max_str = to_persian_numerals("20".to_string());
                format!("{} از {}", value_str, max_str)
            }
        }
    }

    /// Format the grade as a simple value (without context)
    pub fn format_value(&self) -> String {
        match self.locale {
            Locale::En => format!("{:.0}", self.value),
            Locale::Fa => to_persian_numerals(format!("{:.1}", self.value)),
        }
    }

    /// Get the letter grade equivalent
    pub fn letter_grade(&self) -> &'static str {
        let percentage = self.as_percentage();
        match percentage as i32 {
            90..=100 => "A",
            80..=89 => "B",
            70..=79 => "C",
            60..=69 => "D",
            _ => "F",
        }
    }

    /// Get the Farsi equivalent of the letter grade
    pub fn farsi_letter_grade(&self) -> &'static str {
        let percentage = self.as_percentage();
        match percentage as i32 {
            90..=100 => "عالی",    // Excellent
            80..=89 => "خوب",      // Good
            70..=79 => "متوسط",    // Average
            60..=69 => "قابل قبول", // Acceptable
            _ => "مردود",          // Failed
        }
    }

    /// Check if this grade is passing
    pub fn is_passing(&self) -> bool {
        self.as_percentage() >= 60.0
    }
}

/// Convert Western numerals to Persian numerals
pub fn to_persian_numerals(s: String) -> String {
    s.chars()
        .map(|c| match c {
            '0' => '۰',
            '1' => '۱',
            '2' => '۲',
            '3' => '۳',
            '4' => '۴',
            '5' => '۵',
            '6' => '۶',
            '7' => '۷',
            '8' => '۸',
            '9' => '۹',
            '.' => '٫', // Persian decimal separator
            _ => c,
        })
        .collect()
}

/// Convert Persian numerals to Western numerals
pub fn from_persian_numerals(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '۰' => '0',
            '۱' => '1',
            '۲' => '2',
            '۳' => '3',
            '۴' => '4',
            '۵' => '5',
            '۶' => '6',
            '۷' => '7',
            '۸' => '8',
            '۹' => '9',
            '٫' => '.', // Persian decimal separator
            _ => c,
        })
        .collect()
}

/// Parse a grade string in the given locale
pub fn parse_grade(input: &str, locale: Locale) -> Option<LocalizedGrade> {
    // Normalize Persian numerals if present
    let normalized = from_persian_numerals(input.trim());
    
    // Remove common suffixes
    let cleaned = normalized
        .replace('%', "")
        .replace("از", "")
        .replace("۲۰", "")
        .replace("20", "")
        .trim()
        .to_string();

    cleaned.parse::<f64>().ok().map(|value| {
        // Clamp to valid range
        let max = locale.max_grade();
        let clamped = value.clamp(0.0, max);
        LocalizedGrade::new(clamped, locale)
    })
}

/// Validate a grade value for the given locale
pub fn validate_grade(value: f64, locale: Locale) -> Result<(), String> {
    let max = locale.max_grade();
    if value < 0.0 {
        return Err(match locale {
            Locale::En => "Grade cannot be negative".to_string(),
            Locale::Fa => "نمره نمی‌تواند منفی باشد".to_string(),
        });
    }
    if value > max {
        return Err(match locale {
            Locale::En => format!("Grade cannot exceed {}", max),
            Locale::Fa => format!("نمره نمی‌تواند بیشتر از {} باشد", to_persian_numerals(max.to_string())),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grade_conversion() {
        let farsi_grade = LocalizedGrade::farsi(18.0); // 18/20 = 90%
        let english_grade = farsi_grade.convert_to(Locale::En);
        assert!((english_grade.value - 90.0).abs() < 0.01);

        let english_grade = LocalizedGrade::english(75.0); // 75%
        let farsi_grade = english_grade.convert_to(Locale::Fa);
        assert!((farsi_grade.value - 15.0).abs() < 0.01); // 75% = 15/20
    }

    #[test]
    fn test_format_display() {
        let english = LocalizedGrade::english(85.0);
        assert_eq!(english.format_display(), "85%");

        let farsi = LocalizedGrade::farsi(17.0);
        assert_eq!(farsi.format_display(), "۱۷٫۰ از ۲۰");
    }

    #[test]
    fn test_persian_numerals() {
        assert_eq!(to_persian_numerals("123".to_string()), "۱۲۳");
        assert_eq!(from_persian_numerals("۱۲۳"), "123");
    }

    #[test]
    fn test_letter_grades() {
        assert_eq!(LocalizedGrade::english(95.0).letter_grade(), "A");
        assert_eq!(LocalizedGrade::farsi(19.0).letter_grade(), "A"); // 19/20 = 95%
        assert_eq!(LocalizedGrade::farsi(14.0).letter_grade(), "C"); // 14/20 = 70%
    }

    #[test]
    fn test_parse_grade() {
        let grade = parse_grade("85%", Locale::En).unwrap();
        assert_eq!(grade.value, 85.0);

        let grade = parse_grade("۱۷٫۵", Locale::Fa).unwrap();
        assert!((grade.value - 17.5).abs() < 0.01);
    }
}
