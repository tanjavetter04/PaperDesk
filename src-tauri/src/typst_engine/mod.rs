mod noop_progress;
mod world;

pub use world::PaperDeskWorld;

use chrono::{Datelike, Local, Timelike};
use typst::{World, WorldExt};
use typst::diag::{Severity, SourceDiagnostic, Warned};
use typst::foundations::Datetime;
use typst::layout::PagedDocument;
use typst_pdf::{PdfOptions, PdfStandards, Timestamp};

/// Serializable compile diagnostic for the UI.
#[derive(serde::Serialize, Clone, Debug)]
pub struct CompileDiagnostic {
    pub severity: String,
    pub message: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(serde::Serialize, Debug)]
pub struct CompileOutcome {
    pub ok: bool,
    pub pdf_base64: Option<String>,
    pub diagnostics: Vec<CompileDiagnostic>,
}

fn diagnostic_to_dto(world: &PaperDeskWorld, diag: &SourceDiagnostic) -> CompileDiagnostic {
    let (path, line, column) = if let Some(id) = diag.span.id() {
        let path = id
            .vpath()
            .resolve(world.root())
            .map(|p| p.to_string_lossy().into_owned());
        let (line, column) = world
            .range(diag.span)
            .and_then(|range| {
                world
                    .source(id)
                    .ok()?
                    .lines()
                    .byte_to_line_column(range.start)
            })
            .map(|(l, c)| (Some((l + 1) as u32), Some((c + 1) as u32)))
            .unwrap_or((None, None));
        (path, line, column)
    } else {
        (None, None, None)
    };

    CompileDiagnostic {
        severity: match diag.severity {
            Severity::Error => "error".into(),
            Severity::Warning => "warning".into(),
        },
        message: diag.message.to_string(),
        path,
        line,
        column,
    }
}

fn pdf_timestamp() -> Option<Timestamp> {
    let local = Local::now();
    let dt = Datetime::from_ymd_hms(
        local.year(),
        local.month().try_into().ok()?,
        local.day().try_into().ok()?,
        local.hour().try_into().ok()?,
        local.minute().try_into().ok()?,
        local.second().try_into().ok()?,
    )?;
    Timestamp::new_local(dt, local.offset().local_minus_utc() / 60)
}

/// Compile the project entry point to PDF. Resets world caches first.
pub fn compile_to_pdf(
    world: &mut PaperDeskWorld,
) -> Result<(Vec<u8>, Vec<CompileDiagnostic>), Vec<CompileDiagnostic>> {
    world.reset();

    let Warned { output, warnings } = typst::compile::<PagedDocument>(world);

    let mut diagnostics: Vec<_> = warnings
        .iter()
        .map(|w| diagnostic_to_dto(world, w))
        .collect();

    let document = match output {
        Ok(doc) => doc,
        Err(errs) => {
            for e in errs.iter() {
                diagnostics.push(diagnostic_to_dto(world, e));
            }
            return Err(diagnostics);
        }
    };

    let options = PdfOptions {
        timestamp: pdf_timestamp(),
        standards: PdfStandards::default(),
        ..PdfOptions::default()
    };

    match typst_pdf::pdf(&document, &options) {
        Ok(pdf) => Ok((pdf, diagnostics)),
        Err(errs) => {
            for e in errs.iter() {
                diagnostics.push(diagnostic_to_dto(world, e));
            }
            Err(diagnostics)
        }
    }
}
