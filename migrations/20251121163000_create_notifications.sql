-- Create notifications table
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    
    -- Notification content
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    icon VARCHAR(50), -- emoji or icon identifier
    notification_type VARCHAR(50) NOT NULL, -- 'user_registered', 'class_created', 'assignment_submitted', etc.
    
    -- Associated resource (optional)
    resource_type VARCHAR(50), -- 'user', 'class', 'assignment', 'submission', etc.
    resource_id UUID,
    
    -- Status
    is_read BOOLEAN DEFAULT FALSE,
    read_at TIMESTAMP WITH TIME ZONE,
    
    -- Metadata
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Indexes
    CONSTRAINT valid_notification_type CHECK (notification_type IN (
        'user_registered',
        'class_created',
        'class_updated',
        'assignment_created',
        'assignment_submitted',
        'report_generated',
        'profile_change_requested',
        'profile_change_approved',
        'profile_change_rejected',
        'system_announcement'
    ))
);

-- Indexes for performance
CREATE INDEX idx_notifications_user_id ON notifications(user_id);
CREATE INDEX idx_notifications_school_id ON notifications(school_id);
CREATE INDEX idx_notifications_is_read ON notifications(is_read);
CREATE INDEX idx_notifications_created_at ON notifications(created_at DESC);
CREATE INDEX idx_notifications_user_unread ON notifications(user_id, is_read) WHERE is_read = FALSE;

-- Comments
COMMENT ON TABLE notifications IS 'System notifications for users';
COMMENT ON COLUMN notifications.notification_type IS 'Type of notification for filtering and display';
COMMENT ON COLUMN notifications.resource_type IS 'Type of resource this notification relates to';
COMMENT ON COLUMN notifications.resource_id IS 'ID of the resource this notification relates to';
