use indra_mcp::sovereign::write_class::{guard_write, write_class_for, ResourceRef, WriteClass};
use std::path::PathBuf;

#[test]
fn source_documents_are_sealed() {
    let r = ResourceRef::SourceDocument(PathBuf::from("/data/sources/NDT_2026_08.pdf"));
    assert_eq!(write_class_for(&r), WriteClass::Sealed);
    assert!(
        guard_write(&r).is_err(),
        "a Sealed resource accepted a write"
    );
}

#[test]
fn deliverables_are_versioned_and_writable() {
    let r = ResourceRef::Deliverable(PathBuf::from("/data/out/Approval_Note.docx"));
    assert_eq!(write_class_for(&r), WriteClass::Versioned);
    assert!(guard_write(&r).is_ok());
}

#[test]
fn systems_of_record_are_gated_not_writable() {
    let r = ResourceRef::SystemOfRecord {
        system: "sap_pm".to_string(),
    };
    assert_eq!(write_class_for(&r), WriteClass::Gated);
    assert!(
        guard_write(&r).is_err(),
        "a Gated system must receive a proposal, never a direct write"
    );
}
