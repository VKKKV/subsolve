/// Hill-climbing solver for simple substitution ciphers.
///
/// Uses random restart hill climbing with quadgram scoring.
/// Each restart starts with a random key and iteratively
/// swaps pairs of letters, keeping changes that improve the score.
use rand::seq::SliceRandom;
use rand::Rng;

use crate::dict::Dictionary;
use crate::ngram::NgramModel;

const ALPHABET: [u8; 26] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// A solved result
#[derive(Clone)]
pub struct Solution {
    pub plaintext: String,
    pub mapping: [u8; 26],
    pub score: f64,
    pub dict_score: f64,
}

pub struct Solver<'a> {
    cipher_clean: Vec<u8>,
    cipher_orig: String,
    scorer: NgramModel,
    dict: Option<&'a Dictionary>,
    patristocrat: bool,
}

impl<'a> Solver<'a> {
    pub fn new(
        ciphertext: &str,
        scorer: NgramModel,
        dict: Option<&'a Dictionary>,
        patristocrat: bool,
    ) -> Self {
        let cipher_orig = ciphertext.to_string();
        let cipher_clean: Vec<u8> = ciphertext
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .map(|c| c.to_ascii_uppercase() as u8)
            .collect();

        Self {
            cipher_clean,
            cipher_orig,
            scorer,
            dict,
            patristocrat,
        }
    }

    fn decrypt(&self, key: &[u8; 26]) -> Vec<u8> {
        self.cipher_clean
            .iter()
            .map(|&c| key[(c - b'A') as usize])
            .collect()
    }

    /// Apply key preserving original formatting
    fn decrypt_preserve(&self, key: &[u8; 26]) -> String {
        let plain: String = self
            .cipher_orig
            .chars()
            .map(|c| {
                if c.is_ascii_alphabetic() {
                    let idx = c.to_ascii_uppercase() as usize - b'A' as usize;
                    let p = key[idx] as char;
                    if c.is_uppercase() {
                        p
                    } else {
                        p.to_ascii_lowercase()
                    }
                } else {
                    c
                }
            })
            .collect();

        if self.patristocrat {
            // Strip all non-alpha, then try to segment
            let clean: String = plain.chars().filter(|c| c.is_ascii_alphabetic()).collect();
            if let Some(dict) = self.dict {
                if let Some(segmented) = dict.segment(&clean) {
                    return segmented.join::<&str>(" ");
                }
            }
        }
        plain
    }

    /// Combined score: quadgram score + optional dictionary bonus
    fn combined_score(&self, text: &str, quad_score: f64) -> f64 {
        match self.dict {
            Some(dict) => {
                let ds = dict.score_text(text);
                quad_score + ds * 50.0 // dictionary bonus weight
            }
            None => quad_score,
        }
    }

    fn build_key_from_clues(clues: &str) -> ([u8; 26], [bool; 26]) {
        let mut key = ALPHABET;
        let mut locked = [false; 26];

        for clue in clues.split(&[',', ';', ' ']) {
            let clue = clue.trim();
            if clue.len() < 3 {
                continue;
            }
            let (c_str, p_str) = if let Some(pos) = clue.find('=') {
                (&clue[..pos], &clue[pos + 1..])
            } else if let Some(pos) = clue.find("->") {
                (&clue[..pos], &clue[pos + 2..])
            } else if let Some(pos) = clue.find(':') {
                (&clue[..pos], &clue[pos + 1..])
            } else {
                continue;
            };

            let c = c_str.trim().to_ascii_uppercase();
            let p = p_str.trim().to_ascii_uppercase();
            if c.len() != 1 || p.len() != 1 {
                continue;
            }
            let ci = c.as_bytes()[0];
            let pi = p.as_bytes()[0];
            if !ci.is_ascii_uppercase() || !pi.is_ascii_uppercase() {
                continue;
            }

            let idx = (ci - b'A') as usize;
            key[idx] = pi;
            locked[idx] = true;
        }
        (key, locked)
    }

    /// Random restart hill climbing, returns top N solutions
    pub fn solve(
        &self,
        restarts: usize,
        iterations: usize,
        clues: &str,
        top_n: usize,
        verbose: bool,
    ) -> Vec<Solution> {
        let mut all_solutions: Vec<Solution> = Vec::new();
        let mut rng = rand::thread_rng();
        let (initial_key, locked) = Self::build_key_from_clues(clues);

        for run in 0..restarts {
            let mut key = ALPHABET;
            for i in 0..26 {
                if locked[i] {
                    key[i] = initial_key[i];
                }
            }
            let mut unlocked_letters: Vec<u8> = (0..26)
                .map(|i| ALPHABET[i])
                .filter(|&letter| !(0..26).any(|j| locked[j] && initial_key[j] == letter))
                .collect();
            unlocked_letters.shuffle(&mut rng);

            let mut ul_idx = 0;
            for i in 0..26 {
                if !locked[i] {
                    key[i] = unlocked_letters[ul_idx];
                    ul_idx += 1;
                }
            }

            let decrypted = self.decrypt(&key);
            let dec_str = String::from_utf8(decrypted).unwrap();
            let quad_score = self.scorer.score(&dec_str);
            let mut current_score = self.combined_score(&dec_str, quad_score);

            for _step in 0..iterations {
                let mut candidates: Vec<usize> = (0..26).filter(|&i| !locked[i]).collect();
                if candidates.len() < 2 {
                    break;
                }
                let a_idx = rng.gen_range(0..candidates.len());
                let a = candidates[a_idx];
                candidates.swap_remove(a_idx);
                let b = candidates[rng.gen_range(0..candidates.len())];

                key.swap(a, b);
                let child_dec = self.decrypt(&key);
                let child_str = String::from_utf8(child_dec).unwrap();
                let child_quad = self.scorer.score(&child_str);
                let child_score = self.combined_score(&child_str, child_quad);

                if child_score > current_score {
                    current_score = child_score;
                } else {
                    key.swap(a, b);
                }
            }

            let plaintext = self.decrypt_preserve(&key);
            let dec_clean: String = plaintext
                .chars()
                .filter(|c| c.is_ascii_alphabetic())
                .collect();
            let _final_quad = self.scorer.score(&dec_clean);
            let dict_s = self
                .dict
                .map(|d: &Dictionary| d.score_text(&dec_clean))
                .unwrap_or(0.0);

            all_solutions.push(Solution {
                plaintext,
                mapping: key,
                score: current_score,
                dict_score: dict_s,
            });

            // Keep only top N
            if all_solutions.len() > top_n * 3 {
                all_solutions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
                all_solutions.truncate(top_n * 3);
            }

            if verbose {
                let preview: String = all_solutions[0]
                    .plaintext
                    .chars()
                    .filter(|c| c.is_ascii_alphabetic())
                    .take(60)
                    .collect();
                eprintln!(
                    "Run {:2}/{} | score: {:7.2} | {}",
                    run + 1,
                    restarts,
                    all_solutions[0].score,
                    preview
                );
            }
        }

        // Sort and return top N
        all_solutions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        all_solutions.truncate(top_n);
        all_solutions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ngram::NgramModel;

    fn small_model() -> NgramModel {
        NgramModel::from_str("TION 1000\nNTHE 2000\nTHER 500\nTHAT 300\nTHEY 200\n").unwrap()
    }

    fn test_dict() -> Dictionary {
        Dictionary::from_str("the\nquick\nbrown\nfox\njumps\nover\nlazy\ndog\nhello\nworld\n")
    }

    #[test]
    fn test_build_key_from_clues() {
        let (key, locked) = Solver::build_key_from_clues("E=T, G=R");
        assert_eq!(key[(b'E' - b'A') as usize], b'T');
        assert_eq!(key[(b'G' - b'A') as usize], b'R');
        assert!(locked[(b'E' - b'A') as usize]);
        assert!(locked[(b'G' - b'A') as usize]);
    }

    #[test]
    fn test_decrypt_preserves_formatting() {
        let model = small_model();
        let solver = Solver::new("Hello, World!", model, None, false);
        let mut key = ALPHABET;
        for i in 0..13 {
            key.swap(i, i + 13);
        }
        let result = solver.decrypt_preserve(&key);
        assert_eq!(result, "Uryyb, Jbeyq!");
    }

    #[test]
    fn test_solver_basic() {
        let model = small_model();
        let solver = Solver::new("NTHE", model, None, false);
        let sols = solver.solve(5, 100, "", 3, false);
        assert!(!sols.is_empty());
        assert!(!sols[0].plaintext.is_empty());
    }

    #[test]
    fn test_solver_top_n() {
        let model = small_model();
        let solver = Solver::new("NTHETHER", model, None, false);
        let sols = solver.solve(10, 200, "", 5, false);
        assert_eq!(sols.len(), 5);
    }

    #[test]
    fn test_solver_with_dict() {
        let model = small_model();
        let dict = test_dict();
        let solver = Solver::new("NTHETHER", model, Some(&dict), false);
        let sols = solver.solve(5, 100, "", 3, false);
        assert!(!sols.is_empty());
    }
}
