-- Create enum for request status
DO $$ BEGIN
    CREATE TYPE pcr_status AS ENUM ('PENDING', 'APPROVED', 'REJECTED');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Create profile_change_requests table
CREATE TABLE IF NOT EXISTS profile_change_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    payload_diff JSONB NOT NULL,
    requested_by UUID NOT NULL REFERENCES users(id),
    status pcr_status NOT NULL DEFAULT 'PENDING',
    decided_by UUID REFERENCES users(id),
    decided_at TIMESTAMPTZ,
    rejection_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create indexes
CREATE INDEX IF NOT EXISTS profile_change_requests_user_id_idx ON profile_change_requests(user_id);
CREATE INDEX IF NOT EXISTS profile_change_requests_status_idx ON profile_change_requests(status);
CREATE INDEX IF NOT EXISTS profile_change_requests_school_id_idx ON profile_change_requests(user_id); -- Indirectly useful, but maybe join with users is better

-- Add trigger for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_profile_change_requests_updated_at
    BEFORE UPDATE ON profile_change_requests
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
