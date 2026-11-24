mod dict;
mod ngram;
mod solver;
mod vigenere;

use clap::Parser;
use std::io::IsTerminal;
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
    #[arg(long, value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..))]
    key_len: Option<usize>,

    /// Max key length to try for Vigenere (default: 20)
    #[arg(long, default_value = "20")]
    max_key_len: usize,

    /// Disable automatic word segmentation of output
    #[arg(long)]
    no_segment: bool,

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

    /// Group letters into blocks of N (default 5 if no value given)
    #[arg(
        long,
        alias = "group5",
        default_missing_value = "5",
        num_args = 0..=1,
        require_equals = false,
        value_name = "N"
    )]
    group: Option<String>,

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
    let mut group_size = None;
    let mut group_consumed_input = None;

    if let Some(value) = &cli.group {
        match value.parse::<usize>() {
            Ok(0) => {
                eprintln!("error: group size must be at least 1");
                std::process::exit(1);
            }
            Ok(n) => group_size = Some(n),
            Err(_) => {
                group_size = Some(5);
                group_consumed_input = Some(value.clone());
            }
        }
    }

    if group_consumed_input.is_some() && cli.ciphertext.is_some() {
        eprintln!("error: ambiguous input: quote the ciphertext or use --group 5");
        std::process::exit(1);
    }

    // Read input text
    let input = if let Some(path) = &cli.file {
        std::fs::read_to_string(path)?
    } else if let Some(text) = &cli.ciphertext {
        text.clone()
    } else if let Some(text) = group_consumed_input {
        text
    } else if !std::io::stdin().is_terminal() {
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
    if let Some(n) = group_size {
        println!("{}", vigenere::groupn(input, n));
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

    // === Frequency analysis (exit early, not a solve mode) ===
    if cli.analyze {
        print_analysis(input);
        return Ok(());
    }

    // Load quadgram model
    let scorer = match &cli.quadgrams {
        Some(path) if path.exists() => ngram::NgramModel::from_file(path)?,
        Some(path) => {
            eprintln!(
                "warning: quadgram file '{}' not found, using embedded model",
                path.display()
            );
            ngram::NgramModel::from_str(include_str!("../data/english_quadgrams.txt"))?
        }
        _ => ngram::NgramModel::from_str(include_str!("../data/english_quadgrams.txt"))?,
    };

    let output_dict = if cli.no_segment {
        None
    } else {
        Some(dict::Dictionary::from_str(include_str!(
            "../data/words.txt"
        )))
    };

    let top_n = cli.top.max(1);

    // === Vigenere mode ===
    if cli.vigenere {
        let solver = vigenere::VigenereSolver::new(input, &scorer);
        let solutions = solver.solve(cli.max_key_len, cli.key_len, top_n, cli.verbose);

        if solutions.is_empty() {
            eprintln!("error: no Vigenere solutions found");
            std::process::exit(1);
        }

        for sol in &solutions {
            let (plaintext, alternatives) =
                prepare_output_text(&sol.plaintext, output_dict.as_ref(), cli.uppercase);
            let key = if cli.uppercase {
                sol.key.clone()
            } else {
                sol.key.to_lowercase()
            };

            println!("Key: {} | Score: {:.2} (quadgram)", key, sol.score);
            print_solution_body(&plaintext, &alternatives, cli.verbose);
        }
        return Ok(());
    }

    // === Substitution mode ===
    let dict = if let Some(path) = &cli.dict {
        if path.exists() {
            Some(dict::Dictionary::from_file(&path.to_string_lossy())?)
        } else {
            eprintln!("warning: dictionary file not found: {}", path.display());
            None
        }
    } else if cli.patristocrat {
        Some(dict::Dictionary::from_str(include_str!(
            "../data/words.txt"
        )))
    } else {
        None
    };

    let solver = solver::Solver::new(input, scorer, dict.as_ref(), cli.patristocrat);
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

    for (i, sol) in solutions.iter().enumerate() {
        if i > 0 {
            println!();
        }
        let (plaintext, alternatives) =
            prepare_output_text(&sol.plaintext, output_dict.as_ref(), cli.uppercase);
        println!("Score: {:.2} (quadgram)", sol.quad_score);
        print_solution_body(&plaintext, &alternatives, cli.verbose);
        if cli.verbose {
            if dict.is_some() {
                println!("Dict coverage: {:.1}%", sol.dict_score * 100.0);
            }
            println!("Mapping (cipher→plain):");
            for i in 0..26 {
                let c = (b'A' + i as u8) as char;
                let p = sol.mapping[i] as char;
                if c != p {
                    println!("  {} → {}", c, p);
                }
            }
        }
    }

    Ok(())
}

fn segment_and_rank(clean: &str, dict: &dict::Dictionary) -> Vec<String> {
    dict.segment_nbest(clean, 5)
        .unwrap_or_default()
        .into_iter()
        .take(3)
        .map(|words| words.join(" "))
        .collect()
}

fn prepare_output_text(
    plaintext: &str,
    dict: Option<&dict::Dictionary>,
    uppercase: bool,
) -> (String, Vec<String>) {
    let clean: String = plaintext
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect();
    let segments = dict
        .map(|dict| segment_and_rank(&clean, dict))
        .unwrap_or_default();

    let text = segments.first().map(String::as_str).unwrap_or(plaintext);
    let text = apply_output_case(text, uppercase);
    let alternatives = segments
        .iter()
        .skip(1)
        .map(|text| apply_output_case(text, uppercase))
        .collect();

    (text, alternatives)
}

fn apply_output_case(text: &str, uppercase: bool) -> String {
    if uppercase {
        text.to_uppercase()
    } else {
        text.to_lowercase()
    }
}

fn print_solution_body(plaintext: &str, alternatives: &[String], verbose: bool) {
    let sep = output_separator();
    println!("{sep}");
    println!("{}", word_wrap(plaintext, 72));
    println!("{sep}");
    let max_alt = if verbose { 120 } else { 80 };
    for (si, line) in alternatives.iter().enumerate() {
        let short: String = line.chars().take(max_alt).collect();
        let suffix = if line.chars().count() > max_alt {
            "..."
        } else {
            ""
        };
        println!("  alt {}: {}{}", si + 2, short, suffix);
    }
}

fn output_separator() -> String {
    "─".repeat(65)
}

fn print_analysis(ciphertext: &str) {
    let letters: Vec<char> = ciphertext
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    let total = letters.len() as f64;
    if total == 0.0 {
        eprintln!("error: no ASCII letters to analyze");
        std::process::exit(1);
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
    sorted.sort_by_key(|a| std::cmp::Reverse(a.0));

    println!(
        "=== Frequency Analysis ===\nTotal letters: {}\n\nCipher letter frequencies:",
        total as usize
    );
    println!("Letter     Count    Freq%   English rank");
    let dashes = "-".repeat(42);
    println!("{dashes}");
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

fn word_wrap(text: &str, width: usize) -> String {
    let mut result = String::new();
    let mut line_len = 0;
    for word in text.split_whitespace() {
        if line_len + word.len() + 1 > width && line_len > 0 {
            result.push('\n');
            line_len = 0;
        }
        if line_len > 0 {
            result.push(' ');
            line_len += 1;
        }
        result.push_str(word);
        line_len += word.len();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_and_rank_prefers_dp_order() {
        let dict = dict::Dictionary::from_str(include_str!("../data/words.txt"));
        let segments = segment_and_rank("THEQUICKBROWNFOXJUMPSOVERTHELAZYDOG", &dict);
        assert_eq!(segments[0], "the quick brown fox jumps over the lazy dog");
    }

    #[test]
    fn test_ic_random_vs_english() {
        let random = "QWERTYUIOPASDFGHJKLZXCVBNMQWERTYUIOPASDF";
        let english = "THISISASAMPLESENTENCETHATSHOULDHAVEHIGHERIC";
        let ic_rand = index_of_coincidence(
            &random
                .chars()
                .map(|c| c.to_ascii_uppercase())
                .collect::<Vec<_>>(),
        );
        let ic_eng = index_of_coincidence(
            &english
                .chars()
                .map(|c| c.to_ascii_uppercase())
                .collect::<Vec<_>>(),
        );
        assert!(ic_eng > ic_rand);
    }
}
