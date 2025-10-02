// Extractors for different document formats

pub mod pdf;
pub mod latex;

pub use pdf::PdfExtractor;
pub use latex::LatexExtractor;
