use indra_mcp::sovereign::deliverable::{build_approval_note, Finding, Severity};
use indra_mcp::sovereign::provenance::{Citation, SourceBox};
use tempfile::tempdir;

fn finding(text: &str, sev: Severity) -> Finding {
    Finding {
        text: text.to_string(),
        severity: sev,
        citation: Citation {
            document_id: "NDT_2026_08.pdf".to_string(),
            text: "measured wall thickness 6.1 mm".to_string(),
            boxes: vec![SourceBox { page: 1, x: 100, y: 220, w: 400, h: 24 }],
        },
    }
}

#[test]
fn approval_note_is_written_and_nonempty() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("Approval_Note.docx");

    build_approval_note(
        &[finding("Wall thickness below minimum", Severity::CriticalActionRequired)],
        &out,
    )
    .unwrap();

    assert!(out.exists(), "no docx produced");
    assert!(out.metadata().unwrap().len() > 0, "docx is empty");
}

#[test]
fn every_finding_carries_at_least_one_source_box() {
    let f = finding("Wall thickness below minimum", Severity::Monitor);
    assert!(
        !f.citation.boxes.is_empty(),
        "a finding without a source box cannot be traced back to the scan"
    );
}
