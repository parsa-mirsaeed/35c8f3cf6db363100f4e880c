-- Material embeddings tracking table
-- Tracks which materials have been vectorized and stored in Qdrant

CREATE TABLE IF NOT EXISTS material_embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    material_id UUID NOT NULL REFERENCES class_materials(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
    chunks_count INTEGER NOT NULL DEFAULT 0,
    qdrant_collection VARCHAR(100) NOT NULL DEFAULT 'edutalent_materials',
    error_message TEXT,
    processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_material_embedding UNIQUE (material_id)
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_material_embeddings_material_id ON material_embeddings(material_id);
CREATE INDEX IF NOT EXISTS idx_material_embeddings_status ON material_embeddings(status);

-- Trigger for updated_at
CREATE TRIGGER update_material_embeddings_updated_at
    BEFORE UPDATE ON material_embeddings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE material_embeddings IS 'Tracks vectorization status for class materials stored in Qdrant';
COMMENT ON COLUMN material_embeddings.status IS 'Processing status: pending, processing, completed, failed';
COMMENT ON COLUMN material_embeddings.chunks_count IS 'Number of text chunks generated from the material';
COMMENT ON COLUMN material_embeddings.qdrant_collection IS 'Name of the Qdrant collection where vectors are stored';
