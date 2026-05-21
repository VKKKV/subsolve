mod dict;
mod ngram;
mod solver;
mod vigenere;

use clap::Parser;
use std::path::PathBuf;

/// subsolve — offline cryptogram solver
///
/// Supports substitution ciphers (quadgram HC + dictionary) and
/// Vigenere ciphers (Kasiski/IC + chi-squared + quadgram refinement).
/// All modes work offline with embedded language models.
#[derive(Parser)]
#[command(name = "subsolve", version, about)]
struct Cli {
    /// Ciphertext to solve (omit to read from stdin)
    ciphertext: Option<String>,

    /// Read ciphertext from file
    #[arg(short = 'f', long)]
    file: Option<PathBuf>,

    /// Mode: vigenere (default: substitution)
    #[arg(long)]
    vigenere: bool,

    /// Known key length for Vigenere (auto-detect if unset)
    #[arg(long)]
    key_len: Option<usize>,

    /// Max key length to try for Vigenere (default: 20)
    #[arg(long, default_value = "20")]
    max_key_len: usize,

    // === Substitution cipher options ===
    #[arg(short = 'r', long, default_value = "20")]
    restarts: usize,

    #[arg(short = 'i', long, default_value = "2000")]
    iterations: usize,

    #[arg(long)]
    clues: Option<String>,

    #[arg(long)]
    dict: Option<PathBuf>,

    #[arg(long)]
    patristocrat: bool,

    #[arg(short = 'n', long, default_value = "1")]
    top: usize,

    // === Utilities ===
    #[arg(long)]
    scramble: bool,

    #[arg(long)]
    group5: bool,

    #[arg(long)]
    ungroup: bool,

    #[arg(long, aliases = &["upper"])]
    uppercase: bool,

    #[arg(long, aliases = &["lower"])]
    lowercase: bool,

    // === Common ===
    #[arg(long)]
    quadgrams: Option<PathBuf>,

    #[arg(short = 'v', long)]
    verbose: bool,

    #[arg(long)]
    analyze: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Read input text
    let input = if let Some(path) = &cli.file {
        std::fs::read_to_string(path)?
    } else if let Some(text) = &cli.ciphertext {
        text.clone()
    } else if !atty::is(atty::Stream::Stdin) {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        buf
    } else {
        eprintln!("error: provide input as argument, via --file, or pipe to stdin");
        std::process::exit(1);
    };

    let input = input.trim();
    if input.is_empty() {
        eprintln!("error: empty input");
        std::process::exit(1);
    }

    // === Utility modes (run before any solving) ===
    if cli.scramble {
        println!("{}", vigenere::scramble(input));
        return Ok(());
    }
    if cli.group5 {
        println!("{}", vigenere::group5(input));
        return Ok(());
    }
    if cli.ungroup {
        println!("{}", vigenere::ungroup(input));
        return Ok(());
    }
    if cli.uppercase {
        println!("{}", input.to_uppercase());
        return Ok(());
    }
    if cli.lowercase {
        println!("{}", input.to_lowercase());
        return Ok(());
    }

    // === Frequency analysis ===
    if cli.analyze {
        print_analysis(input);
    }

    // Load quadgram model
    let scorer = match &cli.quadgrams {
        Some(path) if path.exists() => ngram::NgramModel::from_file(path)?,
        _ => ngram::NgramModel::from_str(include_str!("../data/english_quadgrams.txt"))?,
    };

    // === Vigenere mode ===
    if cli.vigenere {
        let solver = vigenere::VigenereSolver::new(input, &scorer);
        let solutions = solver.solve(cli.max_key_len, cli.key_len, cli.verbose);

        if solutions.is_empty() {
            eprintln!("error: no Vigenere solutions found");
            std::process::exit(1);
        }

        let top_n = if cli.top > 0 { cli.top } else { 1 };
        for (i, sol) in solutions.iter().enumerate().take(top_n) {
            if top_n > 1 {
                println!("--- Solution {} (score: {:.2}) ---", i, sol.score);
                println!("Key (len={}): {}", sol.key_len, sol.key);
            }
            println!("{}", sol.plaintext);
            if cli.verbose && i == 0 {
                println!();
                println!("Key: {} (length {})", sol.key, sol.key_len);
                println!("Score: {:.2}", sol.score);
            }
        }
        return Ok(());
    }

    // === Substitution mode ===
    let word_list_data;
    let dict = if let Some(path) = &cli.dict {
        if path.exists() {
            word_list_data = dict::Dictionary::from_file(path.to_str().unwrap())?;
            Some(&word_list_data)
        } else {
            eprintln!("warning: dictionary file not found: {}", path.display());
            None
        }
    } else if cli.patristocrat {
        word_list_data = dict::Dictionary::from_str(include_str!("../data/words.txt"));
        Some(&word_list_data)
    } else {
        None
    };

    let solver = solver::Solver::new(input, scorer, dict, cli.patristocrat);
    let top_n = if cli.top > 0 { cli.top } else { 1 };
    let solutions = solver.solve(
        cli.restarts,
        cli.iterations,
        cli.clues.as_deref().unwrap_or(""),
        top_n,
        cli.verbose,
    );

    if solutions.is_empty() {
        eprintln!("error: no solutions found");
        std::process::exit(1);
    }

    if top_n == 1 {
        println!("{}", solutions[0].plaintext);
        if cli.verbose {
            println!();
            println!("Score: {:.2}", solutions[0].score);
            if dict.is_some() {
                println!("Dict coverage: {:.1}%", solutions[0].dict_score * 100.0);
            }
            println!("Mapping (cipher→plain):");
            for i in 0..26 {
                let c = (b'A' + i as u8) as char;
                let p = solutions[0].mapping[i] as char;
                if c != p {
                    println!("  {} → {}", c, p);
                }
            }
        }
    } else {
        for (i, sol) in solutions.iter().enumerate() {
            println!("--- Solution {} (score: {:.2}) ---", i, sol.score);
            if dict.is_some() {
                println!("Dict coverage: {:.1}%", sol.dict_score * 100.0);
            }
            println!("{}", sol.plaintext);
            println!();
        }
    }

    Ok(())
}

fn print_analysis(ciphertext: &str) {
    let letters: Vec<char> = ciphertext
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    let total = letters.len() as f64;
    if total == 0.0 {
        return;
    }

    let mut counts = [0usize; 26];
    for &ch in &letters {
        counts[(ch as u8 - b'A') as usize] += 1;
    }

    let english_order = b"ETAOINSHRDLCUMWFGYPBVKJXQZ";
    let mut sorted: Vec<(usize, char)> = counts
        .iter()
        .enumerate()
        .map(|(i, &c)| (c, (b'A' + i as u8) as char))
        .collect();
    sorted.sort_by(|a, b| b.0.cmp(&a.0));

    println!("=== Frequency Analysis ===");
    println!("Total letters: {}", total as usize);
    println!();
    println!("Cipher letter frequencies:");
    println!("{:8}  {:6}  {:6}  {}", "Letter", "Count", "Freq%", "English rank");
    println!("{}", "-".repeat(45));
    for (count, ch) in &sorted {
        if *count == 0 {
            continue;
        }
        let freq = *count as f64 / total * 100.0;
        let eng = english_order
            .iter()
            .position(|&e| e == *ch as u8)
            .map(|i| format!("#{}", i + 1))
            .unwrap_or_default();
        println!("{:4}     {:6}  {:5.2}%  {}", ch, count, freq, eng);
    }
    println!();
    let ic = index_of_coincidence(&letters);
    println!("IC (Index of Coincidence): {:.4}", ic);
    println!("Expected English IC: ~0.0667, Random IC: ~0.0385");
    if ic < 0.050 {
        println!("Hint: IC is low — could be Vigenere (try --vigenere)");
    }
}

fn index_of_coincidence(letters: &[char]) -> f64 {
    let n = letters.len() as f64;
    if n <= 1.0 {
        return 0.0;
    }
    let mut counts = [0usize; 26];
    for &ch in letters {
        counts[(ch as u8 - b'A') as usize] += 1;
    }
    let sum: usize = counts
        .iter()
        .map(|&c| if c > 0 { c * (c - 1) } else { 0 })
        .sum();
    sum as f64 / (n * (n - 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ic_random_vs_english() {
        let random = "QWERTYUIOPASDFGHJKLZXCVBNMQWERTYUIOPASDF";
        let english = "THISISASAMPLESENTENCETHATSHOULDHAVEHIGHERIC";
        let ic_rand = index_of_coincidence(
            &random.chars().map(|c| c.to_ascii_uppercase()).collect::<Vec<_>>(),
        );
        let ic_eng = index_of_coincidence(
            &english.chars().map(|c| c.to_ascii_uppercase()).collect::<Vec<_>>(),
        );
        assert!(ic_eng > ic_rand);
    }
}
