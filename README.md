# subsolve

Offline cryptogram solver — substitution and Vigenere ciphers.

Inspired by [quipqiup.com](https://quipqiup.com/) — designed for offline CTF use.
Single binary, zero runtime dependencies. Embedded language models (~7 MB).

## Features

| Mode | Flag | Description |
|------|------|-------------|
| Substitution | *(default)* | Quadgram hill climbing + random restart |
| Vigenere | `--vigenere` | Kasiski/IC key-length detection + χ² per-column + quadgram refinement |
| Patristocrat | `--patristocrat` | Substitution + Viterbi word segmentation |
| Dictionary | `--dict <file>` | Bonus scoring from custom word list |
| Frequency analysis | `--analyze` | Letter counts, IC, English rank comparison |
| Clues | `--clues "E=T,G=R"` | Lock known cipher-to-plain mappings |
| Top N | `-n 5` | Return multiple candidate solutions |

**Utility:**
- `--scramble` — random substitution (generate ciphertext)
- `--group [N]` / `--group5` / `--ungroup` — group into blocks of N (default 5) / ungroup
- `--upper` / `--lower` — case conversion
- `--no-segment` — disable automatic word segmentation

**Output features (since v0.2):**
- Auto-lowercase with word segmentation and word-wrap
- N-best segmentation alternatives with per-alt quadgram scores
- Verbose mode (`-v`): sorted key-candidates table with `← best` marker

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
```

Default output: lowercase, word-segmented, with alternatives.

### Solve from file / stdin

```bash
subsolve -f ciphertext.txt
echo "WKH TXLFN EURZQ IRA" | subsolve
```

### Frequency analysis (detect cipher type)

```bash
subsolve --analyze "WKH TXLFN EURZQ IRA"
```

Shows letter frequencies, Index of Coincidence (IC), and a hint if Vigenere is suspected.

### Vigenere solver

```bash
subsolve --vigenere "JGRZQ OQHCV JGGZQ"
subsolve --vigenere --key-len 5 "JGRZQ OQHCV JGGZQ"    # known key length
subsolve --vigenere -v "JGRZQ OQHCV JGGZQ"               # verbose: see all candidates
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
1. Load embedded English quadgram frequency table (389K entries, log-probabilities)
2. Random restart hill climbing — each restart shuffles the key, then iteratively swaps letter pairs to maximize quadgram log-probability
3. When `--dict` is active: quadgram score + dictionary coverage bonus
4. Output: Viterbi DP word segmentation + quadgram re-ranking → best + 2 alternatives

**Vigenere mode:**
1. Kasiski examination + Index of Coincidence → candidate key lengths
2. χ² minimization per column → initial shift values
3. Quadgram scoring across full decrypted text
4. Hill-climbing refinement per key position (3 rounds max)
5. Output: lowercase, Viterbi segmentation + quadgram re-ranking

## References

### Projects & tools

- **[quipqiup](https://quipqiup.com/)** (Edwin Olson) — fast web-based cryptogram solver. The core UX inspiration: show multiple candidates with scores, default-lowercase output.
- **[wordninja](https://github.com/keredson/wordninja)** (keredson, MIT) — probabilistic word splitting using Wikipedia unigram frequencies and Zipf's law. The reference implementation for our Viterbi segmentation cost function.
- **[Practical Cryptography](http://practicalcryptography.com/)** (James Lyons) — comprehensive resource on classical cipher cryptanalysis. The quadgram model, Kasiski examination, χ² test, and hill-climbing algorithm are all adapted from this site.
- **[dcode.fr](https://www.dcode.fr/)** — online cipher toolkit. Reference for Vigenere IC-based key length detection.
- **[CrypTool](https://www.cryptool.org/)** — classical cryptography education software.

### Data sources

- **english_quadgrams.txt** (389K entries, 3.2 MB) — English quadgram log-probabilities from practicalcryptography.com. Embedded at compile time.
- **words.txt** (126K words, frequency-ordered) — from [wordninja](https://github.com/keredson/wordninja) (MIT), based on English Wikipedia unigram frequencies. Each word's position in the list determines its cost via Zipf's law: `cost = ln((rank+1) * ln(N))`. Used for Viterbi word segmentation and embedded at compile time.

### Classical references

- **Kasiski, F. W.** (1863). *Die Geheimschriften und die Dechiffrir-Kunst*. — original publication of the Kasiski examination method for determining Vigenere key length.
- **Friedman, W. F.** (1922). *The Index of Coincidence and Its Applications in Cryptography*. — mathematical foundation for IC-based cipher analysis.
- **Gaines, H. F.** (1939). *Cryptanalysis: A Study of Ciphers and Their Solution*. — classical reference covering substitution, Vigenere, and advanced techniques. Still the go-to book for pencil-and-paper cryptanalysis.

## License

GPLv3 — see [LICENSE](LICENSE).
