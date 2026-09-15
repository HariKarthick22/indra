use crate::computercontroller::{FindingParams, InspectionSeverity, SourceBoxParams};
use crate::sovereign::deliverable::{build_approval_note, Finding, Severity};
use crate::sovereign::ingest::ocr_page;
use crate::sovereign::provenance::{Citation, SourceBox};
use rmcp::model::{ContentBlock, ErrorCode, ErrorData};
use std::path::Path;

impl From<InspectionSeverity> for Severity {
    fn from(s: InspectionSeverity) -> Self {
        match s {
            InspectionSeverity::Normal => Severity::Normal,
            InspectionSeverity::Monitor => Severity::Monitor,
            InspectionSeverity::CriticalActionRequired => Severity::CriticalActionRequired,
        }
    }
}

impl From<SourceBoxParams> for SourceBox {
    fn from(b: SourceBoxParams) -> Self {
        SourceBox {
            page: b.page,
            x: b.x,
            y: b.y,
            w: b.w,
            h: b.h,
        }
    }
}

pub async fn ocr_extract(image_path: &str) -> Result<Vec<ContentBlock>, ErrorData> {
    let lines = ocr_page(Path::new(image_path))
        .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

    if lines.is_empty() {
        return Ok(vec![ContentBlock::text(
            "No text detected in this image.".to_string(),
        )]);
    }

    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        out.push_str(&format!(
            "[{i}] page {} @ ({},{}) {}x{}: {}\n",
            line.bbox.page, line.bbox.x, line.bbox.y, line.bbox.w, line.bbox.h, line.text
        ));
    }
    Ok(vec![ContentBlock::text(out)])
}

pub async fn generate_approval_note(
    output_path: &str,
    findings: Vec<FindingParams>,
) -> Result<Vec<ContentBlock>, ErrorData> {
    if findings.is_empty() {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "At least one finding is required".to_string(),
            None,
        ));
    }

    let built: Vec<Finding> = findings
        .into_iter()
        .map(|f| Finding {
            text: f.text,
            severity: f.severity.into(),
            citation: Citation {
                document_id: f.document_id,
                text: f.quoted_text,
                boxes: f.boxes.into_iter().map(Into::into).collect(),
            },
        })
        .collect();

    build_approval_note(&built, Path::new(output_path))
        .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

    Ok(vec![ContentBlock::text(format!(
        "Approval note written to {output_path} with {} finding(s).",
        built.len()
    ))])
}
