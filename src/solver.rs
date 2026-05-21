/// Hill-climbing solver for simple substitution ciphers.
///
/// Uses random restart hill climbing with quadgram scoring.
/// Each restart starts with a random key and iteratively
/// swaps pairs of letters, keeping changes that improve the score.

use rand::seq::SliceRandom;
use rand::Rng;

use crate::ngram::NgramModel;

const ALPHABET: [u8; 26] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// A solved result
pub struct Solution {
    pub plaintext: String,
    pub mapping: [u8; 26],
    pub score: f64,
}

pub struct Solver {
    cipher_clean: Vec<u8>,
    cipher_orig: String,
    scorer: NgramModel,
}

impl Solver {
    pub fn new(ciphertext: &str, scorer: NgramModel) -> Self {
        let cipher_orig = ciphertext.to_string();
        let cipher_clean: Vec<u8> = ciphertext
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .map(|c| c.to_ascii_uppercase() as u8)
            .collect();

        Self { cipher_clean, cipher_orig, scorer }
    }

    fn decrypt(&self, key: &[u8; 26]) -> Vec<u8> {
        self.cipher_clean
            .iter()
            .map(|&c| key[(c - b'A') as usize])
            .collect()
    }

    /// Apply key preserving original formatting (case, spaces, punctuation)
    fn decrypt_preserve(&self, key: &[u8; 26]) -> String {
        self.cipher_orig
            .chars()
            .map(|c| {
                if c.is_ascii_alphabetic() {
                    let idx = c.to_ascii_uppercase() as usize - b'A' as usize;
                    let p = key[idx] as char;
                    if c.is_uppercase() { p } else { p.to_ascii_lowercase() }
                } else {
                    c
                }
            })
            .collect()
    }

    /// Build initial key from clues like "E=T,G=R".
    /// Returns (key, locked_mask) where locked positions won't be swapped.
    fn build_key_from_clues(clues: &str) -> ([u8; 26], [bool; 26]) {
        let mut key = ALPHABET;
        let mut locked = [false; 26];

        for clue in clues.split(|c| c == ',' || c == ';') {
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

    /// Random restart hill climbing
    pub fn solve(
        &self,
        restarts: usize,
        iterations: usize,
        clues: &str,
        verbose: bool,
    ) -> Solution {
        let mut best_key = [0u8; 26];
        let mut best_score = f64::NEG_INFINITY;
        let mut best_plaintext = String::new();
        let mut rng = rand::thread_rng();
        let (initial_key, locked) = Self::build_key_from_clues(clues);

        for run in 0..restarts {
            // Build key: start from identity, apply locked clues, shuffle rest
            let mut key = ALPHABET;
            // First, apply locked positions
            for i in 0..26 {
                if locked[i] {
                    key[i] = initial_key[i];
                }
            }
            // Fill unlocked positions with remaining letters, shuffled
            let mut unlocked_letters: Vec<u8> = (0..26)
                .map(|i| ALPHABET[i])
                .filter(|&letter| {
                    // letter not used by any locked position
                    !(0..26).any(|j| locked[j] && initial_key[j] == letter)
                })
                .collect();
            unlocked_letters.shuffle(&mut rng);

            let mut ul_idx = 0;
            for i in 0..26 {
                if !locked[i] {
                    key[i] = unlocked_letters[ul_idx];
                    ul_idx += 1;
                }
            }

            let mut current_score =
                self.scorer.score(&String::from_utf8(self.decrypt(&key)).unwrap());

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
                let decrypted = self.decrypt(&key);
                let child_score =
                    self.scorer.score(&String::from_utf8(decrypted).unwrap());

                if child_score > current_score {
                    current_score = child_score;
                } else {
                    key.swap(a, b);
                }
            }

            if current_score > best_score {
                best_score = current_score;
                best_key = key;
                best_plaintext = self.decrypt_preserve(&key);

                if verbose {
                    let preview: String = best_plaintext
                        .chars()
                        .filter(|c| c.is_ascii_alphabetic())
                        .take(60)
                        .collect();
                    eprintln!(
                        "Run {:2}/{} | score: {:7.2} | {}",
                        run + 1,
                        restarts,
                        best_score,
                        preview
                    );
                }
            }
        }

        Solution { plaintext: best_plaintext, mapping: best_key, score: best_score }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ngram::NgramModel;

    fn small_model() -> NgramModel {
        NgramModel::from_str("TION 1000\nNTHE 2000\nTHER 500\nTHAT 300\nTHEY 200\n").unwrap()
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
        let solver = Solver::new("Hello, World!", model);
        let mut key = ALPHABET;
        for i in 0..13 {
            key.swap(i, i + 13);
        }
        let result = solver.decrypt_preserve(&key);
        assert_eq!(result, "Uryyb, Jbeyq!");
    }

    #[test]
    fn test_solver_basic() {
        // A very short ciphertext that the small model can handle
        let model = small_model();
        let solver = Solver::new("NTHE", model);
        let sol = solver.solve(5, 100, "", false);
        // Should at least produce something
        assert!(!sol.plaintext.is_empty());
    }
}
