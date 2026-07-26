-- Add extracted_text column to class_materials for storing content from uploaded files
-- This allows vectorization of directly uploaded files without URL storage

ALTER TABLE class_materials 
ADD COLUMN IF NOT EXISTS extracted_text TEXT;

COMMENT ON COLUMN class_materials.extracted_text IS 'Pre-extracted text content from uploaded files for vectorization';
