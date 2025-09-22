use anyhow::Result;
use tiktoken_rs::cl100k_base;

const CHUNK_SIZE_TOKENS: usize = 1000;
const CHUNK_OVERLAP_TOKENS: usize = 100;

pub fn chunk_text(text: &str) -> Result<Vec<String>> {
    let bpe = cl100k_base()?;
    let tokens = bpe.encode_with_special_tokens(text);

    let mut chunks = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let end = std::cmp::min(i + CHUNK_SIZE_TOKENS, tokens.len());
        let chunk_tokens = &tokens[i..end];
        let chunk_text = bpe.decode(chunk_tokens.to_vec())?;
        chunks.push(chunk_text);

        if end == tokens.len() {
            break;
        }

        i += CHUNK_SIZE_TOKENS - CHUNK_OVERLAP_TOKENS;
    }

    Ok(chunks)
}
