// Extractors for different document formats

pub mod pdf;
pub mod latex;
pub mod txt;
pub mod docx;

pub use pdf::PdfExtractor;
pub use latex::LatexExtractor;
pub use txt::TxtExtractor;
pub use docx::DocxExtractor;
