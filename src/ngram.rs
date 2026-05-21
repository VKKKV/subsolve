/// Quadgram language model for scoring English text.
///
/// Loads a quadgram frequency table (e.g., from practicalcryptography.com)
/// and computes log-probability scores for arbitrary text.
use std::collections::HashMap;
use std::path::Path;

const N: usize = 4;

pub struct NgramModel {
    scores: HashMap<u32, f64>,
    floor: f64,
    _total: u64,
}

impl NgramModel {
    /// Load from the standard quadgram file format: "QUAD <count>" per line
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read_to_string(path.as_ref())?;
        Self::from_str(&data)
    }

    /// Load from an embedded string (include_str!)
    pub fn from_str(data: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut ngrams: HashMap<u32, u64> = HashMap::new();
        let mut total: u64 = 0;

        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() != 2 {
                continue;
            }
            let quad_str = parts[0].trim();
            if quad_str.len() != N {
                continue;
            }
            let count: u64 = match parts[1].trim().parse() {
                Ok(c) => c,
                Err(_) => continue,
            };
            let key = quad_to_key(quad_str);
            ngrams.insert(key, count);
            total += count;
        }

        if ngrams.is_empty() {
            return Err("no valid quadgrams found in input".into());
        }

        let total_f = total as f64;
        let mut scores: HashMap<u32, f64> = HashMap::with_capacity(ngrams.len());
        for (key, count) in &ngrams {
            scores.insert(*key, (*count as f64 / total_f).log10());
        }

        // Floor: simulate seeing 0.01 of a count for any unseen quadgram
        let floor = (0.01 / total_f).log10();

        Ok(Self {
            scores,
            floor,
            _total: total,
        })
    }

    /// Score a piece of text. Higher is more English-like.
    /// Text should be A-Z uppercase only, no spaces/punctuation.
    pub fn score(&self, text: &str) -> f64 {
        let bytes = text.as_bytes();
        if bytes.len() < N {
            return 0.0;
        }

        let mut total = 0.0;
        let end = bytes.len() - (N - 1);
        for i in 0..end {
            let key = bytes_to_key(&bytes[i..i + N]);
            total += self.scores.get(&key).copied().unwrap_or(self.floor);
        }
        total
    }
}

/// Pack 4 uppercase ASCII bytes into a u32 (each byte uses 5 bits, total 20 bits)
/// 'A' = 0, 'B' = 1, ..., 'Z' = 25
#[inline]
fn quad_to_key(s: &str) -> u32 {
    let b = s.as_bytes();
    debug_assert!(b.len() == 4);
    ((b[0] - b'A') as u32) << 15
        | ((b[1] - b'A') as u32) << 10
        | ((b[2] - b'A') as u32) << 5
        | ((b[3] - b'A') as u32)
}

/// Same but from a byte slice (for scoring hot path)
#[inline]
fn bytes_to_key(b: &[u8]) -> u32 {
    debug_assert!(b.len() >= 4);
    ((b[0] - b'A') as u32) << 15
        | ((b[1] - b'A') as u32) << 10
        | ((b[2] - b'A') as u32) << 5
        | ((b[3] - b'A') as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_encoding() {
        assert_eq!(quad_to_key("AAAA"), 0);
        assert_eq!(quad_to_key("ZZZZ"), 0b11001_11001_11001_11001);
        assert_eq!(quad_to_key("THEY"), quad_to_key("THEY"));
        assert_eq!(bytes_to_key(b"THEY"), quad_to_key("THEY"));
        assert_ne!(quad_to_key("AAAA"), quad_to_key("AAAB"));
    }

    #[test]
    fn test_score_texts_same_length() {
        // Use a larger model
        let data = "\
TION 5000000
NTHE 4000000
THER 3000000
THAT 2000000
WITH 2000000
HAVE 1500000
THIS 1500000
";
        let model = NgramModel::from_str(data).unwrap();

        // Both same length (6 chars = 3 quadgrams)
        // "THAT" is in model, "XQZW" is not
        let text_with_match = "THATXX";
        let all_random = "XQZWVM";
        assert!(
            model.score(text_with_match) > model.score(all_random),
            "text with known quadgram should score higher"
        );
    }

    #[test]
    fn test_empty_or_short_text() {
        let data = "TION 1000\n";
        let model = NgramModel::from_str(data).unwrap();
        assert_eq!(model.score(""), 0.0);
        assert_eq!(model.score("AB"), 0.0);
        assert_eq!(model.score("ABC"), 0.0);
    }

    #[test]
    fn test_real_quadgrams_beat_random() {
        // With the real 389K-entry model, English text must score higher
        let model = NgramModel::from_str(include_str!("../data/english_quadgrams.txt")).unwrap();

        let english = "THEQUICKBROWNFOXJUMPSOVERTHELAZYDOG";
        let random = "QHGKDLVWMZPTFJRYXNCBOSAEIUGQHFKDRYXP";
        assert!(
            model.score(english) > model.score(random),
            "real english should score higher with full model"
        );
    }
}
