use indra_mcp::sovereign::ingest::ocr_page;
use std::path::Path;

#[test]
fn ocr_returns_lines_with_nonzero_boxes() {
    let fixture = Path::new("tests/fixtures/sample_inspection.png");
    if !fixture.exists() {
        eprintln!("fixture missing; skipping");
        return;
    }

    let lines = ocr_page(fixture).unwrap();

    assert!(!lines.is_empty(), "OCR returned no lines");
    for line in &lines {
        assert!(
            line.bbox.w > 0 && line.bbox.h > 0,
            "degenerate box: {:?}",
            line.bbox
        );
    }
}

#[test]
fn ocr_text_is_searchable() {
    let fixture = Path::new("tests/fixtures/sample_inspection.png");
    if !fixture.exists() {
        return;
    }
    let lines = ocr_page(fixture).unwrap();
    let all: String = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(!all.trim().is_empty(), "OCR produced no text");
}
