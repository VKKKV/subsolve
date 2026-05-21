mod ngram;
mod solver;

use clap::Parser;
use std::path::PathBuf;

/// subsolve — offline substitution cipher solver
///
/// Uses quadgram frequency analysis + hill climbing with random restarts.
/// No external dependencies required after build.
#[derive(Parser)]
#[command(name = "subsolve", version, about)]
struct Cli {
    /// Ciphertext to solve (omit to read from stdin)
    ciphertext: Option<String>,

    /// Read ciphertext from file
    #[arg(short = 'f', long)]
    file: Option<PathBuf>,

    /// Number of random restarts (default: 20)
    #[arg(short = 'r', long, default_value = "20")]
    restarts: usize,

    /// Iterations per restart (default: 2000)
    #[arg(short = 'i', long, default_value = "2000")]
    iterations: usize,

    /// Known letter mappings: "E=T,G=R" or "E=T,G->R"
    #[arg(long)]
    clues: Option<String>,

    /// Path to quadgram data file
    #[arg(long, default_value = "/dev/null")]
    quadgrams: Option<PathBuf>,

    /// Show progress per restart
    #[arg(short = 'v', long)]
    verbose: bool,

    /// Show frequency analysis of ciphertext
    #[arg(long)]
    analyze: bool,

    /// Show all top solutions instead of just the best
    #[arg(long)]
    top: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Read ciphertext
    let ciphertext = if let Some(path) = &cli.file {
        std::fs::read_to_string(path)?
    } else if let Some(text) = &cli.ciphertext {
        text.clone()
    } else if !atty::is(atty::Stream::Stdin) {
        // Read from piped stdin
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        buf
    } else {
        eprintln!("error: provide ciphertext as argument, via --file, or pipe to stdin");
        std::process::exit(1);
    };

    let ciphertext = ciphertext.trim();

    if ciphertext.is_empty() {
        eprintln!("error: empty ciphertext");
        std::process::exit(1);
    }

    // Frequency analysis
    if cli.analyze {
        print_analysis(ciphertext);
    }

    // Load quadgram model
    let scorer = match &cli.quadgrams {
        Some(path) if path.exists() && path != &PathBuf::from("/dev/null") => {
            ngram::NgramModel::from_file(path)?
        }
        _ => {
            // Use embedded data
            ngram::NgramModel::from_str(include_str!("../data/english_quadgrams.txt"))?
        }
    };

    // Solve
    let solver = solver::Solver::new(ciphertext, scorer);
    let solution = solver.solve(
        cli.restarts,
        cli.iterations,
        cli.clues.as_deref().unwrap_or(""),
        cli.verbose,
    );

    // Output
    if cli.analyze {
        println!();
    }
    println!("{}", solution.plaintext);

    // Show mapping if verbose
    if cli.verbose {
        println!();
        println!("Score: {:.2}", solution.score);
        println!("Mapping (cipher→plain):");
        for i in 0..26 {
            let c = (b'A' + i as u8) as char;
            let p = solution.mapping[i] as char;
            if c != p {
                println!("  {} → {}", c, p);
            }
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

    // Count frequencies
    let mut counts = [0usize; 26];
    for &ch in &letters {
        counts[(ch as u8 - b'A') as usize] += 1;
    }

    // English letter frequency order (ETAOIN SHRDLU...)
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
    println!("{:8}  {:6}  {:7}  {}", "Letter", "Count", "Freq%", "English");
    println!("{}", "-".repeat(50));
    for (count, ch) in &sorted {
        let freq = *count as f64 / total * 100.0;
        let eng = english_order
            .iter()
            .position(|&e| e == *ch as u8)
            .map(|i| format!("#{}", i + 1))
            .unwrap_or_default();
        println!("{:4}     {:6}  {:6.2}%  {}", ch, count, freq, eng);
    }
    println!();
    println!("IC (Index of Coincidence): {:.4}", index_of_coincidence(&letters));
    println!("Expected English IC: ~0.0667, Random IC: ~0.0385");
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
    let sum: usize = counts.iter().map(|&c| if c > 0 { c * (c - 1) } else { 0 }).sum();
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
            &random.chars().map(|c| c.to_ascii_uppercase()).collect::<Vec<_>>()
        );
        let ic_eng = index_of_coincidence(
            &english.chars().map(|c| c.to_ascii_uppercase()).collect::<Vec<_>>()
        );
        assert!(ic_eng > ic_rand);
    }
}
