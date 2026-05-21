# subsolve

Offline substitution cipher solver with automatic frequency analysis.

Inspired by [quipqiup.com](https://quipqiup.com/) — designed for offline CTF use.

Single binary, zero runtime dependencies.

## Features

| Mode | Flag | Description |
|------|------|-------------|
| Substitution | *(default)* | Quadgram hill climbing with random restart |
| Vigenere | `--vigenere` | IC/Kasiski key-length detection + χ² per-column |
| Patristocrat | `--patristocrat` | Same-substitution + Viterbi word segmentation |
| Dictionary | `--dict <file>` | Bonus scoring from custom word list |
| Frequency analysis | `--analyze` | Letter counts, IC, English rank comparison |
| Clues | `--clues "E=T,G=R"` | Lock known cipher-to-plain mappings |
| Top N | `-n 5` | Return multiple candidate solutions |

**Utility:**
- `--scramble` — random substitution
- `--group [N]` / `--group5` / `--ungroup` — grouping in blocks of N (default 5) / ungrouping
- `--upper` / `--lower` — case conversion

## Install

### From source

```bash
git clone https://git.k174.dev/k/subsolve.git
cd subsolve
cargo build --release
# binary at target/release/subsolve
```

### From GitHub releases

Download the pre-built binary for your platform from the [releases page](https://git.k174.dev/k/subsolve).

## Usage

### Solve a substitution cipher

```bash
subsolve "WKH TXLFN EURZQ IRA"
# → THE QUICK BROWN FOX
```

### Solve from file

```bash
subsolve -f ciphertext.txt
```

### Solve from stdin

```bash
echo "WKH TXLFN EURZQ IRA" | subsolve
```

### Frequency analysis (detect cipher type)

```bash
subsolve --analyze "WKH TXLFN EURZQ IRA"
```

Output includes letter frequencies, Index of Coincidence, and a hint if Vigenere is suspected.

### Vigenere solver

```bash
subsolve --vigenere "JGRZQ OQHCV JGGZQ"
```

With a known key length:

```bash
subsolve --vigenere --key-len 5 "JGRZQ OQHCV JGGZQ"
```

### Patristocrat (no word boundaries)

```bash
subsolve --patristocrat "WKHQWKHTXLFNEURZQIRAMXPSVRYHUWKHODCBGRJ"
```

### Dictionary-assisted solving

```bash
subsolve --dict /usr/share/dict/words "WKH TXLFN EURZQ IRA"
```

### Clues (partial known mapping)

```bash
subsolve --clues "E=T,G=R" "WKH TXLFN EURZQ IRA"
```

Separators: `=`, `->`, `:`. Multiple clues: comma or semicolon.

## Algorithm

**Substitution mode:**
1. Loads a 389K-entry English quadgram frequency table
2. Random restart hill climbing — each restart shuffles the key, then iteratively swaps pairs of letters to maximize quadgram log-probability
3. When `--dict` is active, quadgram score + dictionary coverage bonus
4. Patristocrat mode runs Viterbi DP after decryption to insert word boundaries

**Vigenere mode:**
1. Kasiski examination + Index of Coincidence → candidate key lengths
2. χ² minimization per column → initial shift values
3. Scored by quadgram model across the full decrypted text

## License

GPLv3 — see [LICENSE](LICENSE).
