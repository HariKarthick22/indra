use crate::sovereign::provenance::SourceBox;
use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct OcrLine {
    pub text: String,
    pub bbox: SourceBox,
    pub confidence: f32,
}

pub fn ocr_page(image: &Path) -> Result<Vec<OcrLine>> {
    let out = Command::new("tesseract")
        .arg(image)
        .args(["stdout", "tsv"])
        .output()?;

    if !out.status.success() {
        bail!("tesseract exited {}", out.status);
    }

    Ok(parse_tsv(&String::from_utf8_lossy(&out.stdout)))
}

// Tesseract TSV columns: level page_num block_num par_num line_num word_num
// left top width height conf text. Level 4 rows are lines; level 5 are words.
// Words carry the text, so words are accumulated and grouped into their line.
fn parse_tsv(tsv: &str) -> Vec<OcrLine> {
    let mut lines: Vec<OcrLine> = Vec::new();
    let mut current_key: Option<(u32, u32)> = None;

    for row in tsv.lines().skip(1) {
        let f: Vec<&str> = row.split('\t').collect();
        if f.len() < 12 || f[0] != "5" {
            continue;
        }
        let text = f[11].trim();
        let conf: f32 = f[10].parse().unwrap_or(-1.0);
        if text.is_empty() || conf < 0.0 {
            continue;
        }

        let page: u32 = f[1].parse().unwrap_or(1);
        let line_num: u32 = f[4].parse().unwrap_or(0);
        let (x, y, w, h) = (
            f[6].parse().unwrap_or(0),
            f[7].parse().unwrap_or(0),
            f[8].parse().unwrap_or(0),
            f[9].parse().unwrap_or(0),
        );

        if current_key == Some((page, line_num)) {
            let last = lines.last_mut().expect("key set implies a line exists");
            last.text.push(' ');
            last.text.push_str(text);
            let right = last.bbox.x.max(x) + last.bbox.w.max(w);
            last.bbox.x = last.bbox.x.min(x);
            last.bbox.y = last.bbox.y.min(y);
            last.bbox.w = right - last.bbox.x;
            last.bbox.h = last.bbox.h.max(h);
            last.confidence = last.confidence.min(conf);
        } else {
            current_key = Some((page, line_num));
            lines.push(OcrLine {
                text: text.to_string(),
                bbox: SourceBox { page, x, y, w, h },
                confidence: conf,
            });
        }
    }

    lines
}
