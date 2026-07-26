use super::knowledge_asset_repository::KnowledgeAssetStatus;

#[test]
fn status_strings_round_trip() {
    let cases = [
        (KnowledgeAssetStatus::Submitted, "submitted"),
        (KnowledgeAssetStatus::OcrPending, "ocr_pending"),
        (KnowledgeAssetStatus::OcrReady, "ocr_ready"),
        (KnowledgeAssetStatus::EmbeddingPending, "embedding_pending"),
        (KnowledgeAssetStatus::Embedded, "embedded"),
        (KnowledgeAssetStatus::Published, "published"),
        (KnowledgeAssetStatus::Archived, "archived"),
        (KnowledgeAssetStatus::Failed, "failed"),
    ];

    for (status, expected) in cases {
        assert_eq!(status.as_str(), expected);
        assert_eq!(KnowledgeAssetStatus::parse(expected).unwrap(), status);
    }
}

#[test]
fn unknown_status_is_rejected() {
    let error = KnowledgeAssetStatus::parse("processing").unwrap_err();
    assert!(error.to_string().contains("Unknown knowledge asset status"));
}

#[test]
fn serde_uses_snake_case_lifecycle_values() {
    let json = serde_json::to_string(&KnowledgeAssetStatus::EmbeddingPending).unwrap();
    assert_eq!(json, "\"embedding_pending\"");

    let decoded: KnowledgeAssetStatus = serde_json::from_str("\"ocr_ready\"").unwrap();
    assert_eq!(decoded, KnowledgeAssetStatus::OcrReady);
}
