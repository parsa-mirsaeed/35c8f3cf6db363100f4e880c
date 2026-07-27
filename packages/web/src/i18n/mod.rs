//! Internationalization (i18n) Module for EduTalent
//!
//! Provides bilingual support for English and Farsi (Persian) with:
//! - Farsi as the default language
//! - RTL layout support
//! - Locale-specific grading (20-based for Farsi, 100-based for English)
//! - Type-safe translation keys

pub mod grading;
pub mod locale;
pub mod provider;
pub mod translations;

pub use grading::*;
pub use locale::*;
pub use provider::*;
pub use translations::*;
