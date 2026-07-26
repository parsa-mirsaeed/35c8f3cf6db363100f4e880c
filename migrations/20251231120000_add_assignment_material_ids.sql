-- Add material_ids column to assignments table
ALTER TABLE assignments ADD COLUMN material_ids UUID[] DEFAULT '{}';

-- Add comment
COMMENT ON COLUMN assignments.material_ids IS 'List of material IDs associated with this assignment for RAG context';
