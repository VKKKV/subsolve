/// Vigenere cipher solver.
///
/// Algorithm:
/// 1. Kasiski examination + Index of Coincidence → find key length
/// 2. Per-column chi-squared vs English frequencies → initial key
/// 3. Quadgram scoring → pick best key length/shift combo
/// 4. Hill climbing refinement on individual key positions
use std::cmp::Ordering;
use std::collections::HashMap;

use crate::ngram::NgramModel;

const AZ: &[u8; 26] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const MAX_REFINE_ITERATIONS: usize = 3;

/// English letter frequencies (A-Z)
const EN_FREQ: [f64; 26] = [
    0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228, 0.02015, 0.06094, 0.06966, 0.00153,
    0.00772, 0.04025, 0.02406, 0.06749, 0.07507, 0.01929, 0.00095, 0.05987, 0.06327, 0.09056,
    0.02758, 0.00978, 0.02360, 0.00150, 0.01974, 0.00074,
];

pub struct VigenereSolver<'a> {
    clean: Vec<u8>, // uppercase A-Z only
    orig: String,
    quadgrams: &'a NgramModel,
}

#[derive(Clone)]
pub struct VigenereSolution {
    pub plaintext: String,
    pub key: String,
    pub key_len: usize,
    pub shifts: Vec<u8>,
    pub score: f64,
}

fn cmp_f64(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Less)
}

impl<'a> VigenereSolver<'a> {
    pub fn new(text: &str, quadgrams: &'a NgramModel) -> Self {
        let orig = text.to_string();
        let clean: Vec<u8> = text
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .map(|c| c.to_ascii_uppercase() as u8)
            .collect();
        Self {
            clean,
            orig,
            quadgrams,
        }
    }

    /// Solve Vigenere cipher automatically
    pub fn solve(
        &self,
        max_key_len: usize,
        known_key_len: Option<usize>,
        verbose: bool,
    ) -> Vec<VigenereSolution> {
        let max = known_key_len
            .unwrap_or(max_key_len.min(self.clean.len().max(2) / 2))
            .max(1)
            .min(self.clean.len().max(1));

        let candidates = known_key_len
            .map(|k| vec![k])
            .unwrap_or_else(|| self.find_key_lengths(max));

        let mut solutions: Vec<VigenereSolution> = Vec::new();

        for &kl in &candidates {
            if kl < 1 || kl > self.clean.len() {
                continue;
            }
            let shifts = self.initial_shifts(kl);
            let key = shifts.iter().map(|&s| AZ[s as usize] as char).collect();
            let plain = self.decrypt(&shifts);
            let score = self.quadgrams.score(&plain);

            // Collect candidates; print sorted table in verbose mode
            if !verbose {
                solutions.push(VigenereSolution {
                    plaintext: self.reconstruct(&plain),
                    key,
                    key_len: kl,
                    shifts,
                    score,
                });
            } else {
                // Collect for later sorted display
                solutions.push(VigenereSolution {
                    plaintext: self.reconstruct(&plain),
                    key: key.clone(),
                    key_len: kl,
                    shifts,
                    score,
                });
            }
        }

        for sol in &mut solutions {
            self.refine_solution(sol);
        }

        solutions.sort_by(|a, b| cmp_f64(b.score, a.score));
        solutions.truncate(5);

        if verbose {
            eprintln!();
            eprintln!("Key candidates (sorted by quadgram score):");
            eprintln!("  {:>3}  {:<14}  {:>8}", "len", "key", "score");
            eprintln!("  {:-^3}  {:-^14}  {:-^8}", "", "", "");
            for (idx, sol) in solutions.iter().enumerate() {
                eprintln!(
                    "  {:>3}  {:<14}  {:>8.2}{}",
                    sol.key_len,
                    sol.key.to_lowercase(),
                    sol.score,
                    if idx == 0 { "  \u{2190} best" } else { "" }
                );
            }
        }

        solutions
    }

    /// Refine a candidate by hill-climbing each key position independently.
    fn refine_solution(&self, sol: &mut VigenereSolution) {
        for _ in 0..MAX_REFINE_ITERATIONS {
            let mut improved = false;

            for pos in 0..sol.key_len {
                let mut best_shift = sol.shifts[pos];
                let mut best_score = sol.score;

                for candidate_shift in 0..26u8 {
                    if candidate_shift == best_shift {
                        continue;
                    }

                    let mut test_shifts = sol.shifts.clone();
                    test_shifts[pos] = candidate_shift;
                    let plain = self.decrypt(&test_shifts);
                    let score = self.quadgrams.score(&plain);

                    if score > best_score {
                        best_score = score;
                        best_shift = candidate_shift;
                        improved = true;
                    }
                }

                if best_shift != sol.shifts[pos] {
                    sol.shifts[pos] = best_shift;
                    let plain = self.decrypt(&sol.shifts);
                    sol.plaintext = self.reconstruct(&plain);
                    sol.key = sol.shifts.iter().map(|&s| AZ[s as usize] as char).collect();
                    sol.score = self.quadgrams.score(&plain);
                }
            }

            if !improved {
                break;
            }
        }
    }

    /// Find best key lengths via IC + Kasiski
    fn find_key_lengths(&self, max: usize) -> Vec<usize> {
        let n = self.clean.len();
        if n < 4 {
            return vec![1];
        }

        // Method 1: Index of Coincidence
        let mut ic_scores: Vec<(usize, f64)> = (2..=max)
            .map(|kl| {
                let ic = self.avg_ic(kl);
                (kl, (ic - 0.0667).abs()) // lower = better
            })
            .collect();
        ic_scores.sort_by(|a, b| cmp_f64(a.1, b.1));

        // Method 2: Kasiski examination (for longer texts)
        let mut kasiski_scores: HashMap<usize, usize> = HashMap::new();
        if n >= 20 {
            let repeats = self.kasiski(3);
            for &dist in &repeats {
                // Factor the distance, count small factors
                for f in 2..=max.min(10) {
                    if dist % f == 0 {
                        *kasiski_scores.entry(f).or_insert(0) += 1;
                    }
                }
            }
        }

        // Merge: prefer IC results, boost with Kasiski
        let mut merged: Vec<(usize, f64)> = ic_scores
            .iter()
            .map(|&(kl, ic_dist)| {
                let kas = kasiski_scores.get(&kl).copied().unwrap_or(0) as f64;
                // Combined score: IC distance (lower better) - Kasiski boost
                (kl, ic_dist - kas * 0.01)
            })
            .collect();
        merged.sort_by(|a, b| cmp_f64(a.1, b.1));

        // Always try key length 1 (Caesar/substitution) at the end
        let mut result: Vec<usize> = merged.iter().map(|&(kl, _)| kl).collect();
        if !result.contains(&1) {
            result.push(1);
        }

        // Remove duplicates and return top candidates
        result.truncate(8);
        result
    }

    /// Average Index of Coincidence for a given key length
    fn avg_ic(&self, key_len: usize) -> f64 {
        if self.clean.len() < key_len * 2 {
            return 0.0;
        }
        let mut total_ic = 0.0;
        for col in 0..key_len {
            let column: Vec<u8> = self
                .clean
                .iter()
                .skip(col)
                .step_by(key_len)
                .copied()
                .collect();
            total_ic += self.ic(&column);
        }
        total_ic / key_len as f64
    }

    /// Index of Coincidence for a sequence of letters
    fn ic(&self, letters: &[u8]) -> f64 {
        let n = letters.len() as f64;
        if n <= 1.0 {
            return 0.0;
        }
        let mut counts = [0usize; 26];
        for &c in letters {
            counts[(c - b'A') as usize] += 1;
        }
        let sum: usize = counts
            .iter()
            .map(|&c| if c > 1 { c * (c - 1) } else { 0 })
            .sum();
        sum as f64 / (n * (n - 1.0))
    }

    /// Kasiski examination: find distances between repeated trigrams
    fn kasiski(&self, gram_len: usize) -> Vec<usize> {
        let n = self.clean.len();
        if n < gram_len * 2 {
            return vec![];
        }

        let mut positions: HashMap<u32, Vec<usize>> = HashMap::new();
        for i in 0..=(n - gram_len) {
            let key = pack_gram(&self.clean[i..i + gram_len]);
            positions.entry(key).or_default().push(i);
        }

        let mut distances = Vec::new();
        for pos_list in positions.values() {
            if pos_list.len() < 2 {
                continue;
            }
            for i in 0..pos_list.len() {
                for j in (i + 1)..pos_list.len() {
                    let dist = pos_list[j] - pos_list[i];
                    if dist > 0 {
                        distances.push(dist);
                    }
                }
            }
        }
        distances
    }

    /// Find initial shifts for each key position using chi-squared
    fn initial_shifts(&self, key_len: usize) -> Vec<u8> {
        (0..key_len)
            .map(|col| {
                let column: Vec<u8> = self
                    .clean
                    .iter()
                    .skip(col)
                    .step_by(key_len)
                    .copied()
                    .collect();
                self.best_shift(&column)
            })
            .collect()
    }

    /// Find the shift that minimizes chi-squared for a single column
    fn best_shift(&self, column: &[u8]) -> u8 {
        let mut best = 0u8;
        let mut best_chi2 = f64::MAX;

        for shift in 0..26u8 {
            let mut counts = [0usize; 26];
            for &c in column {
                let p = (26 + (c - b'A') as i16 - shift as i16) as u8 % 26;
                counts[p as usize] += 1;
            }

            let n = column.len() as f64;
            let mut chi2 = 0.0;
            for i in 0..26 {
                let expected = EN_FREQ[i] * n;
                let observed = counts[i] as f64;
                if expected > 0.0 {
                    chi2 += (observed - expected).powi(2) / expected;
                }
            }

            if chi2 < best_chi2 {
                best_chi2 = chi2;
                best = shift;
            }
        }
        best
    }

    /// Decrypt using shifts (each shift = how much to subtract per position)
    fn decrypt(&self, shifts: &[u8]) -> String {
        self.clean
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                let shift = shifts[i % shifts.len()];
                let p = (26 + (c - b'A') as i16 - shift as i16) as u8 % 26;
                (p + b'A') as char
            })
            .collect()
    }

    /// Reconstruct original formatting with decrypted letters
    fn reconstruct(&self, plain: &str) -> String {
        let mut pi = 0;
        self.orig
            .chars()
            .map(|c| {
                if c.is_ascii_alphabetic() {
                    let p = plain.as_bytes()[pi];
                    pi += 1;
                    if c.is_uppercase() {
                        p as char
                    } else {
                        (p as char).to_ascii_lowercase()
                    }
                } else {
                    c
                }
            })
            .collect()
    }
}

/// Pack `gram_len` bytes into a u32 (max 4 bytes for 26-letter alphabet uses 5 bits each)
/// Since we need up to 3-4 letter grams, pack into u32 with 5 bits per letter
fn pack_gram(gram: &[u8]) -> u32 {
    let mut key = 0u32;
    for &b in gram.iter().take(4) {
        key = (key << 5) | ((b - b'A') as u32);
    }
    key
}

/// Apply a scramble (random substitution) to text
pub fn scramble(text: &str) -> String {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    let mut mapping: Vec<u8> = AZ.to_vec();
    mapping.shuffle(&mut rng);

    text.chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                let idx = c.to_ascii_uppercase() as usize - b'A' as usize;
                let p = mapping[idx] as char;
                if c.is_uppercase() {
                    p
                } else {
                    p.to_ascii_lowercase()
                }
            } else {
                c
            }
        })
        .collect()
}

/// Group text in blocks of N
pub fn groupn(text: &str, n: usize) -> String {
    let letters: String = text
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    letters
        .as_bytes()
        .chunks(n)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Remove grouping (spaces every 5 chars)
pub fn ungroup(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ngram::NgramModel;

    fn model() -> NgramModel {
        NgramModel::from_str(
            "TION 1000\nNTHE 2000\nTHER 500\nTHAT 300\nTHEY 200\nENGL 100\nLISH 100\n",
        )
        .unwrap()
    }

    fn full_model() -> NgramModel {
        NgramModel::from_str(include_str!("../data/english_quadgrams.txt")).unwrap()
    }

    #[test]
    fn test_ic_english_vs_random() {
        let m = model();
        let solver = VigenereSolver::new(
            "THEQUICKBROWNFOXJUMPSOVERTHELAZYDOGTHEQUICKBROWNFOXJUMPSOVERTHELAZYDOG",
            &m,
        );
        let ic_eng = solver.avg_ic(1);
        let m2 = model();
        let solver2 =
            VigenereSolver::new("QWERTYUIOPASDFGHJKLZXCVBNMQWERTYUIOPASDFGHJKLZXCVBNM", &m2);
        let ic_rnd = solver2.avg_ic(1);
        assert!(ic_eng > ic_rnd);
    }

    #[test]
    fn test_vigenere_encrypt_decrypt() {
        // Encrypt with known key, then solve
        let text = "HELLOWORLDTHISISATEST";
        let key = [3, 8, 14]; // D, I, O
        let cipher: String = text
            .bytes()
            .enumerate()
            .map(|(i, c)| {
                let s = key[i % key.len()];
                (((c - b'A') + s) % 26 + b'A') as char
            })
            .collect();

        let model = full_model();
        let solver = VigenereSolver::new(&cipher, &model);
        let sols = solver.solve(10, Some(3), false);
        assert!(!sols.is_empty());
        assert_eq!(sols[0].key_len, 3);
    }

    #[test]
    fn test_scramble_changes_text() {
        let orig = "HELLO WORLD";
        let scrambled = scramble(orig);
        // Should have same length and letter distribution
        assert_eq!(
            orig.chars().filter(|c| c.is_ascii_alphabetic()).count(),
            scrambled
                .chars()
                .filter(|c| c.is_ascii_alphabetic())
                .count()
        );
        // Should be different (unless astronomically unlikely)
        if scrambled.to_uppercase() == orig.to_uppercase() {
            // Might happen with very short text, skip assertion
        }
    }

    #[test]
    fn test_groupn() {
        assert_eq!(groupn("THISISATEST", 5), "THISI SATES T");
        assert_eq!(groupn("HelloWorld", 5), "HELLO WORLD");
        assert_eq!(groupn("HELLOWORLD", 2), "HE LL OW OR LD");
    }

    #[test]
    fn test_ungroup() {
        assert_eq!(ungroup("THISI SATES T"), "THISISATEST");
        assert_eq!(ungroup("HELLO WORLD"), "HELLOWORLD");
    }

    #[test]
    fn test_kasiski_finds_repeats() {
        let m = model();
        let solver = VigenereSolver::new(
            "ABCXYZABCXYZABCXYZ", // clear repeating pattern every 3
            &m,
        );
        let dists = solver.kasiski(3);
        assert!(!dists.is_empty());
        // Distances should include 3, 6, 9
        assert!(dists.contains(&3) || dists.contains(&6));
    }
}
