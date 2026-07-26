-- Create user_preferences table for general settings
CREATE TABLE IF NOT EXISTS user_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    
    -- General Settings
    timezone VARCHAR(50) DEFAULT 'UTC',
    language VARCHAR(10) DEFAULT 'en',
    date_format VARCHAR(20) DEFAULT 'YYYY-MM-DD',
    time_format VARCHAR(10) DEFAULT '24h', -- '12h' or '24h'
    
    -- Notification Preferences
    email_notifications BOOLEAN DEFAULT TRUE,
    push_notifications BOOLEAN DEFAULT TRUE,
    in_app_notifications BOOLEAN DEFAULT TRUE,
    
    -- Notification Types
    notify_user_registered BOOLEAN DEFAULT TRUE,
    notify_class_created BOOLEAN DEFAULT TRUE,
    notify_assignment_submitted BOOLEAN DEFAULT TRUE,
    notify_report_generated BOOLEAN DEFAULT TRUE,
    notify_profile_change BOOLEAN DEFAULT TRUE,
    notify_system_announcements BOOLEAN DEFAULT TRUE,
    
    -- Email Digest
    email_digest_frequency VARCHAR(20) DEFAULT 'daily', -- 'never', 'daily', 'weekly'
    
    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Index for quick lookups
CREATE INDEX idx_user_preferences_user_id ON user_preferences(user_id);

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_user_preferences_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_user_preferences_updated_at
    BEFORE UPDATE ON user_preferences
    FOR EACH ROW
    EXECUTE FUNCTION update_user_preferences_updated_at();

-- Comments
COMMENT ON TABLE user_preferences IS 'User preferences for general settings and notifications';
COMMENT ON COLUMN user_preferences.timezone IS 'User timezone (e.g., UTC, America/New_York)';
COMMENT ON COLUMN user_preferences.language IS 'User interface language code (e.g., en, es, fr)';
COMMENT ON COLUMN user_preferences.date_format IS 'Preferred date display format';
COMMENT ON COLUMN user_preferences.email_digest_frequency IS 'How often to send email digests';
