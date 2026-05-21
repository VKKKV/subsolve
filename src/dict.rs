/// Dictionary with word lookup and Viterbi segmentation.
///
/// Supports:
/// - Fast word lookup via HashSet
/// - Word boundary insertion (Viterbi DP) for patristocrat mode
/// - Word frequency scoring for dictionary mode
use std::collections::HashSet;

pub struct Dictionary {
    words: HashSet<String>,
    max_word_len: usize,
}

impl Dictionary {
    /// Load from a newline-separated word list file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read_to_string(path)?;
        Ok(Self::from_str(&data))
    }

    /// Load from a string (embedded or otherwise)
    pub fn from_str(data: &str) -> Self {
        let mut words = HashSet::new();
        let mut max_word_len = 0;

        for line in data.lines() {
            let word = line.trim().to_ascii_lowercase();
            // Remove \r from Windows-style line endings
            let word = word.trim_end_matches('\r').to_string();
            if word.is_empty() {
                continue;
            }
            // Only keep alphabetic words of reasonable length
            if word.chars().all(|c| c.is_ascii_alphabetic()) && word.len() >= 2 {
                let len = word.len();
                if len > max_word_len {
                    max_word_len = len;
                }
                words.insert(word);
            }
        }

        Self {
            words,
            max_word_len,
        }
    }

    /// Check if a word is in the dictionary
    pub fn contains(&self, word: &str) -> bool {
        self.words.contains(word)
    }

    /// Viterbi segmentation: insert spaces into a string of letters
    /// to form valid dictionary words.
    ///
    /// Allows single-letter words (a, i) as fallback.
    /// Returns None if no valid segmentation exists.
    pub fn segment(&self, text: &str) -> Option<Vec<String>> {
        self.segment_nbest(text, 1)
            .map(|segmentations| segmentations.into_iter().next().unwrap())
    }

    /// Return top-N Viterbi segmentations, best score first.
    pub fn segment_nbest(&self, text: &str, n: usize) -> Option<Vec<Vec<String>>> {
        if n == 0 {
            return Some(Vec::new());
        }

        let text = text.to_ascii_lowercase();
        let total_len = text.len();
        if total_len == 0 {
            return Some(vec![vec![]]);
        }

        struct Entry {
            score: f64,
            prev_pos: usize,
            prev_idx: usize,
        }

        let mut dp: Vec<Vec<Entry>> = (0..=total_len).map(|_| Vec::new()).collect();
        dp[0].push(Entry {
            score: 0.0,
            prev_pos: 0,
            prev_idx: 0,
        });

        let max_len = self.max_word_len.min(total_len);

        for i in 0..total_len {
            if dp[i].is_empty() {
                continue;
            }

            let end_max = (i + max_len).min(total_len);
            for entry_idx in 0..dp[i].len() {
                let entry_score = dp[i][entry_idx].score;
                for j in (i + 1)..=end_max {
                    let candidate = &text[i..j];
                    let mut valid = self.contains(candidate);

                    // Also allow single-letter English words
                    if !valid && candidate.len() == 1 {
                        valid = true; // any single letter is valid as fallback
                    }

                    if valid {
                        // Score: prefer known words over sequences of single letters.
                        // Single letters get a penalty, multi-letter words get a bonus
                        // proportional to their length.
                        let word_cost = if candidate.len() == 1 && !self.contains(candidate) {
                            -10.0 // penalize single-letter unknowns
                        } else {
                            (candidate.len() as f64).sqrt() * 4.0 // prefer longer words
                        };
                        let new_score = entry_score + word_cost;
                        let new_entry = Entry {
                            score: new_score,
                            prev_pos: i,
                            prev_idx: entry_idx,
                        };
                        let insert_pos = dp[j]
                            .binary_search_by(|entry| {
                                entry
                                    .score
                                    .partial_cmp(&new_entry.score)
                                    .unwrap_or(std::cmp::Ordering::Less)
                                    .reverse()
                            })
                            .unwrap_or_else(|pos| pos);

                        if dp[j].len() < n || insert_pos < dp[j].len() {
                            dp[j].insert(insert_pos, new_entry);
                            if dp[j].len() > n {
                                dp[j].truncate(n);
                            }
                        }
                    }
                }
            }
        }

        // If no perfect segmentation, fall back to treating unknown words
        // as single characters (worst-case segmentation)
        if dp[total_len].is_empty() {
            return Some(vec![text.chars().map(|c| c.to_string()).collect()]);
        }

        // Backtrack each retained path to build word lists.
        let mut segmentations = Vec::new();
        for result_idx in 0..dp[total_len].len() {
            let mut words = Vec::new();
            let mut pos = total_len;
            let mut idx = result_idx;

            while pos > 0 {
                let entry = dp[pos].get(idx)?;
                words.push(text[entry.prev_pos..pos].to_string());
                pos = entry.prev_pos;
                idx = entry.prev_idx;
            }

            words.reverse();
            if !segmentations.contains(&words) {
                segmentations.push(words);
            }
        }

        Some(segmentations)
    }

    /// Score a text based on what fraction of words are in the dictionary.
    /// Returns a value in [0, 1] where 1.0 = all words known.
    /// For patristocrat text (no spaces), uses Viterbi segmentation first.
    pub fn score_text(&self, text: &str) -> f64 {
        let has_spaces = text.contains(' ');
        let words: Vec<&str> = if has_spaces {
            text.split_whitespace()
                .filter(|w| w.chars().any(|c| c.is_ascii_alphabetic()))
                .collect()
        } else {
            // Patristocrat mode: words aren't separated yet
            // Just check bigrams/trigrams instead
            return self.score_ngram(text);
        };

        if words.is_empty() {
            return 0.0;
        }

        let known = words
            .iter()
            .filter(|w| {
                let clean: String = w.chars().filter(|c| c.is_ascii_alphabetic()).collect();
                !clean.is_empty() && self.contains(&clean.to_ascii_lowercase())
            })
            .count();

        known as f64 / words.len() as f64
    }

    /// Score a string of letters by checking what fraction of n-grams
    /// are valid dictionary substrings. Used for patristocrat scoring.
    fn score_ngram(&self, text: &str) -> f64 {
        let text = text.to_ascii_lowercase();
        let bytes = text.as_bytes();
        if bytes.len() < 2 {
            return 0.0;
        }

        // Check all trigrams - what fraction start valid dictionary words?
        let mut total = 0;
        let mut valid = 0;
        let max_len = self.max_word_len.min(bytes.len());

        for i in 0..bytes.len() {
            for j in (i + 2)..=(i + max_len).min(bytes.len()) {
                total += 1;
                let candidate = &text[i..j];
                if self.contains(candidate) {
                    valid += 1;
                }
            }
            // Limit to avoid O(n^2) blowup
            if total > 5000 {
                break;
            }
        }

        if total == 0 {
            0.0
        } else {
            valid as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dict() -> Dictionary {
        Dictionary::from_str("the\nquick\nbrown\nfox\njumps\nover\nlazy\ndog\nhello\nworld\n")
    }

    #[test]
    fn test_contains() {
        let dict = test_dict();
        assert!(dict.contains("the"));
        assert!(dict.contains("quick"));
        assert!(!dict.contains("xyzzy"));
    }

    #[test]
    fn test_segmentation() {
        let dict = test_dict();
        let result = dict.segment("thequickbrownfox").unwrap();
        assert_eq!(result, vec!["the", "quick", "brown", "fox"]);
    }

    #[test]
    fn test_segment_nbest() {
        let dict = Dictionary::from_str(
            "the
quick
brown
fox
thequick
brownfox
quickbrown
",
        );
        let results = dict.segment_nbest("thequickbrownfox", 3).unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(
            results.first(),
            Some(&vec![
                "the".to_string(),
                "quick".to_string(),
                "brown".to_string(),
                "fox".to_string(),
            ])
        );
        assert!(results.contains(&vec![
            "the".to_string(),
            "quick".to_string(),
            "brownfox".to_string(),
        ]));
        assert!(results.contains(&vec![
            "thequick".to_string(),
            "brown".to_string(),
            "fox".to_string(),
        ]));
    }

    #[test]
    fn test_score_text() {
        let dict = test_dict();
        let score = dict.score_text("the quick brown fox");
        assert!((score - 1.0).abs() < 0.01);

        let score2 = dict.score_text("xyzzz qwerty");
        assert!(score2 < 1.0);
    }
}
