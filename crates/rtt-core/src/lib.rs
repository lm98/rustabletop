mod diagnostics;
mod world;

pub use diagnostics::{Diagnostic, Location, Severity};
pub use world::ProjectWorld;

use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;
use typst_render::RenderOptions;

pub struct Compiled {
    pub document: PagedDocument,
    pub warnings: Vec<Diagnostic>,
}

pub fn compile(world: &ProjectWorld) -> Result<Compiled, Vec<Diagnostic>> {
    let result = typst::compile::<PagedDocument>(world);
    let warnings = result
        .warnings
        .iter()
        .map(|d| Diagnostic::from_typst(world, d))
        .collect();
    match result.output {
        Ok(document) => Ok(Compiled { document, warnings }),
        Err(errors) => Err(errors
            .iter()
            .map(|d| Diagnostic::from_typst(world, d))
            .chain(warnings)
            .collect()),
    }
}

pub fn export_pdf(world: &ProjectWorld, document: &PagedDocument) -> Result<Vec<u8>, Vec<Diagnostic>> {
    typst_pdf::pdf(document, &PdfOptions::default())
        .map_err(|errs| errs.iter().map(|d| Diagnostic::from_typst(world, d)).collect())
}

pub fn export_pngs(document: &PagedDocument, ppi: f32) -> anyhow::Result<Vec<Vec<u8>>> {
    let options = RenderOptions {
        pixel_per_pt: f64::from(ppi / 72.0).into(),
        ..RenderOptions::default()
    };
    document
        .pages()
        .iter()
        .map(|page| {
            typst_render::render(page, &options)
                .encode_png()
                .map_err(|e| anyhow::anyhow!("png encoding failed: {e}"))
        })
        .collect()
}
