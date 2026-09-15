use anyhow::{bail, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteClass {
    Sealed,
    Versioned,
    Gated,
}

/// Known limitation: classification is decided by which variant the *caller*
/// constructs, not by anything intrinsic to the path. Nothing here checks that
/// a path passed as `Deliverable` isn't actually a tracked source document
/// elsewhere on disk — there is no canonical "sealed roots" registry a path
/// is checked against. Not exploitable today because the only live caller,
/// `deliverable::build_approval_note`, always constructs `Deliverable` from
/// its own output-path argument. Revisit before any caller accepts an
/// externally-influenced path for this decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceRef {
    SourceDocument(PathBuf),
    Deliverable(PathBuf),
    SystemOfRecord { system: String },
}

// Exhaustive by construction: a new resource kind cannot be added without the
// compiler demanding its class here, so nothing defaults to writable.
pub fn write_class_for(r: &ResourceRef) -> WriteClass {
    match r {
        ResourceRef::SourceDocument(_) => WriteClass::Sealed,
        ResourceRef::Deliverable(_) => WriteClass::Versioned,
        ResourceRef::SystemOfRecord { .. } => WriteClass::Gated,
    }
}

pub fn guard_write(r: &ResourceRef) -> Result<()> {
    match write_class_for(r) {
        WriteClass::Versioned => Ok(()),
        WriteClass::Sealed => bail!("{r:?} is Sealed; source documents are never modified"),
        WriteClass::Gated => {
            bail!("{r:?} is Gated; emit a proposal for approval rather than writing directly")
        }
    }
}
