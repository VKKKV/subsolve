/// Dictionary with word lookup and Viterbi segmentation.
///
/// Supports:
/// - Fast word lookup via HashMap
/// - Word boundary insertion (Viterbi DP) for patristocrat mode
use std::collections::HashMap;
use std::io;
use std::path::Path;

// Cost near rank 1,000 in the embedded list: favor a custom word over
// most splits into rarer words while keeping common built-in words cheap.
const CUSTOM_WORD_COST: f64 = 9.0;

fn words_from_lines(data: &str) -> impl Iterator<Item = String> + '_ {
    data.lines()
        .map(str::trim)
        .filter(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_alphabetic()))
        .map(str::to_ascii_lowercase)
}

pub struct Dictionary {
    words: HashMap<String, f64>,
    max_word_len: usize,
}

impl Dictionary {
    /// Load a frequency-ordered word list.
    pub fn from_str(data: &str) -> Self {
        let word_list: Vec<_> = words_from_lines(data).collect();
        let max_word_len = word_list.iter().map(String::len).max().unwrap_or(0);

        let total = word_list.len() as f64;
        let log_total = total.ln();
        let mut words = HashMap::new();
        for (i, word) in word_list.into_iter().enumerate() {
            let cost = ((i + 1) as f64 * log_total).ln();
            words.entry(word).or_insert(cost);
        }

        Self {
            words,
            max_word_len,
        }
    }

    /// Add words from an unranked, newline-separated file.
    pub fn merge_file(&mut self, path: &Path) -> io::Result<()> {
        self.merge_custom(&std::fs::read_to_string(path)?);
        Ok(())
    }

    fn merge_custom(&mut self, data: &str) {
        for word in words_from_lines(data) {
            self.max_word_len = self.max_word_len.max(word.len());
            let cost = self.words.entry(word).or_insert(CUSTOM_WORD_COST);
            *cost = cost.min(CUSTOM_WORD_COST);
        }
    }

    /// Check if a word is in the dictionary
    pub fn contains(&self, word: &str) -> bool {
        self.words.contains_key(word)
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
                        // Lower Zipf cost means a more common word, so negate for max-DP.
                        let word_cost = if let Some(cost) = self.words.get(candidate) {
                            -cost
                        } else if candidate.len() == 1 {
                            -20.0
                        } else {
                            unreachable!("segmentation tried non-dict multi-letter word");
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
th
equ
ick
bro
wn
fo
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
    }

    #[test]
    fn test_segmentation_prefers_frequency_over_length() {
        let dict = Dictionary::from_str(
            "a
with
cat
wit
ha
",
        );

        let result = dict.segment("witha").unwrap();
        assert_eq!(result, vec!["with", "a"]);
    }

    #[test]
    fn test_merge_custom_words_for_segmentation() {
        let mut dict = Dictionary::from_str("the\ncode\nth\ne\nblorp\ntastic\n");
        let the_cost = dict.words["the"];
        // Use the real word list for costs comparable to the custom-word constant.
        let mut full_dict = Dictionary::from_str(include_str!("../data/words.txt"));
        full_dict.merge_custom("Blorptastic\ncode\n");
        let results = full_dict.segment_nbest("theblorptasticcode", 3).unwrap();
        assert_eq!(results[0], ["the", "blorptastic", "code"]);

        dict.merge_custom("Blorptastic\nthe\ninvalid word\n");
        assert_eq!(dict.words["the"], the_cost);
        assert_eq!(dict.words["blorptastic"], CUSTOM_WORD_COST);
        assert_eq!(dict.max_word_len, "blorptastic".len());
        assert_eq!(dict.segment("the").unwrap(), ["the"]);
        assert!(!dict.contains("invalid word"));
    }
}
