-- Class materials table for storing class resources
-- Following ISO 15836 (Dublin Core) metadata standards for educational resources

CREATE TABLE IF NOT EXISTS class_materials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    class_section_id UUID NOT NULL REFERENCES class_sections(id) ON DELETE CASCADE,
    title VARCHAR(500) NOT NULL,
    description TEXT,
    material_type VARCHAR(50) NOT NULL CHECK (material_type IN ('document', 'video', 'link', 'image', 'audio', 'other')),
    file_url TEXT,
    file_size_bytes BIGINT,
    mime_type VARCHAR(100),
    external_link TEXT,
    is_required BOOLEAN NOT NULL DEFAULT false,
    display_order INTEGER NOT NULL DEFAULT 0,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_class_materials_class_section_id ON class_materials(class_section_id);
CREATE INDEX IF NOT EXISTS idx_class_materials_created_at ON class_materials(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_class_materials_material_type ON class_materials(material_type);

-- Trigger for updated_at
CREATE TRIGGER update_class_materials_updated_at
    BEFORE UPDATE ON class_materials
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE class_materials IS 'Stores learning materials and resources for class sections (ISO 15836 Dublin Core compliant)';
COMMENT ON COLUMN class_materials.material_type IS 'Type of material: document, video, link, image, audio, other';
COMMENT ON COLUMN class_materials.is_required IS 'Whether this material is mandatory for the course';
