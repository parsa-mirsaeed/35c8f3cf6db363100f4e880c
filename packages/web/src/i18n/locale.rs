//! Locale definitions for EduTalent
//!
//! Supports English and Farsi with Farsi as the default language.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported locales in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Locale {
    /// English (Left-to-Right)
    En,
    /// Farsi/Persian (Right-to-Left) - Default
    Fa,
}

impl Default for Locale {
    fn default() -> Self {
        // Farsi is the default language as per requirement
        Locale::Fa
    }
}

impl Locale {
    /// Get the ISO 639-1 language code
    pub fn code(&self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Fa => "fa",
        }
    }

    /// Get the text direction for this locale
    pub fn direction(&self) -> Direction {
        match self {
            Locale::En => Direction::Ltr,
            Locale::Fa => Direction::Rtl,
        }
    }

    /// Get the HTML dir attribute value
    pub fn dir_attr(&self) -> &'static str {
        self.direction().as_str()
    }

    /// Get the display name in the locale's own language
    pub fn native_name(&self) -> &'static str {
        match self {
            Locale::En => "English",
            Locale::Fa => "فارسی",
        }
    }

    /// Get the maximum grade value for this locale's grading system
    pub fn max_grade(&self) -> f64 {
        match self {
            Locale::En => 100.0,
            Locale::Fa => 20.0,
        }
    }

    /// Check if this locale uses RTL text direction
    pub fn is_rtl(&self) -> bool {
        matches!(self, Locale::Fa)
    }

    /// Parse locale from string code
    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_lowercase().as_str() {
            "en" | "english" => Some(Locale::En),
            "fa" | "farsi" | "persian" | "فارسی" => Some(Locale::Fa),
            _ => None,
        }
    }

    /// Get all supported locales
    pub fn all() -> &'static [Locale] {
        &[Locale::Fa, Locale::En]
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Text direction for layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Left-to-Right (English, etc.)
    Ltr,
    /// Right-to-Left (Farsi, Arabic, etc.)
    Rtl,
}

impl Direction {
    /// Get the HTML dir attribute value
    pub fn as_str(&self) -> &'static str {
        match self {
            Direction::Ltr => "ltr",
            Direction::Rtl => "rtl",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_locale_is_farsi() {
        assert_eq!(Locale::default(), Locale::Fa);
    }

    #[test]
    fn test_locale_codes() {
        assert_eq!(Locale::En.code(), "en");
        assert_eq!(Locale::Fa.code(), "fa");
    }

    #[test]
    fn test_rtl_detection() {
        assert!(!Locale::En.is_rtl());
        assert!(Locale::Fa.is_rtl());
    }

    #[test]
    fn test_grade_scales() {
        assert_eq!(Locale::En.max_grade(), 100.0);
        assert_eq!(Locale::Fa.max_grade(), 20.0);
    }

    #[test]
    fn test_from_code() {
        assert_eq!(Locale::from_code("en"), Some(Locale::En));
        assert_eq!(Locale::from_code("fa"), Some(Locale::Fa));
        assert_eq!(Locale::from_code("farsi"), Some(Locale::Fa));
        assert_eq!(Locale::from_code("unknown"), None);
    }
}
