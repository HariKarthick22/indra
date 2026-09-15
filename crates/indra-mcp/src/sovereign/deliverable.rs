use crate::sovereign::provenance::Citation;
use crate::sovereign::write_class::{guard_write, ResourceRef};
use anyhow::Result;
use docx_rs::{Docx, Paragraph, Run};
use std::fs::File;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Normal,
    Monitor,
    CriticalActionRequired,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Normal => "Normal",
            Severity::Monitor => "Monitor",
            Severity::CriticalActionRequired => "Critical Action Required",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub text: String,
    pub severity: Severity,
    pub citation: Citation,
}

pub fn build_approval_note(findings: &[Finding], out: &Path) -> Result<()> {
    guard_write(&ResourceRef::Deliverable(out.to_path_buf()))?;

    let mut doc = Docx::new().add_paragraph(
        Paragraph::new().add_run(Run::new().add_text("Inspection Approval Note").bold()),
    );

    for f in findings {
        doc = doc.add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text(format!("[{}] ", f.severity.label())).bold())
                .add_run(Run::new().add_text(&f.text)),
        );

        let refs: Vec<String> = f
            .citation
            .boxes
            .iter()
            .map(|b| format!("p.{} @ ({},{})", b.page, b.x, b.y))
            .collect();

        doc = doc.add_paragraph(Paragraph::new().add_run(
            Run::new()
                .add_text(format!("Source: {} — {}", f.citation.document_id, refs.join("; ")))
                .italic(),
        ));
    }

    doc.build().pack(File::create(out)?)?;
    Ok(())
}
