# subsolve

Offline cryptogram solver — substitution and Vigenere ciphers.

Inspired by [quipqiup.com](https://quipqiup.com/) — designed for offline CTF use.
Single binary, zero runtime dependencies. Embedded language models (~4.2 MB).

## Features

| Mode               | Flag                | Description                                                           |
| ------------------ | ------------------- | --------------------------------------------------------------------- |
| Substitution       | _(default)_         | Quadgram hill climbing + random restart                               |
| Vigenere           | `--vigenere`        | Kasiski/IC key-length detection + χ² per-column + quadgram refinement |
| Patristocrat       | `--patristocrat`    | Substitution + Viterbi word segmentation                              |
| Dictionary         | `--dict <file>`     | Add custom words to output segmentation                               |
| Frequency analysis | `--analyze`         | Letter counts, IC, English rank comparison                            |
| Clues              | `--clues "E=T,G=R"` | Lock known cipher-to-plain mappings                                   |
| Top N              | `-n 5`              | Return multiple candidate solutions                                   |

**Utility:**

- `--scramble` — random substitution (generate ciphertext)
- `--group [N]` / `--group5` / `--ungroup` — group into blocks of N (default 5) / ungroup
- `--upper` / `--lower` — case conversion (text utilities; cannot be combined with solver options)
- `--no-segment` — disable automatic word segmentation

**Output features:**

- Auto-lowercase with word segmentation and word-wrap
- N-best segmentation alternatives
- Verbose mode (`-v`): Vigenere prints a sorted key-candidate table with `← best`; substitution prints per-run progress and the key mapping

## Install

### From source

```bash
git clone https://git.k174.dev/k/subsolve.git
cd subsolve
cargo build --release
# binary at target/release/subsolve
```

Or install it to `~/.cargo/bin` with `cargo install --path .`.

## Usage

All examples use the same public-domain passage, encrypted three ways.

### Solve a substitution cipher

```bash
subsolve -f examples/substitution.txt
```

Default output is lowercase and word-segmented, with alternatives. The solver wraps the plaintext at 72 columns. The first three plaintext lines are shown here.

```text
Score: -1751.81 (quadgram)
─────────────────────────────────────────────────────────────────
walt whitman has somewhere a fine and just distinction between loving by
allowance and loving with personal love this distinction applies to
books as well as to men and women and in the case of the not very
```

### Solve from file / stdin

```bash
subsolve -f examples/substitution.txt
cat examples/substitution.txt | subsolve
```

### Frequency analysis (detect cipher type)

```bash
subsolve --analyze -f examples/vigenere.txt
```

Shows letter frequencies, Index of Coincidence (IC), and a hint if Vigenere is suspected. The example above is the Vigenere ciphertext, so its low IC triggers the hint.

### Vigenere solver

```bash
subsolve --vigenere -f examples/vigenere.txt
subsolve --vigenere --key-len 5 -f examples/vigenere.txt   # known key length
subsolve --vigenere -v -f examples/vigenere.txt            # verbose: see all candidates
```

The example ciphertext uses the key `LEMON`; auto-detection recovers it:

```text
Key: lemon | Score: -1751.81 (quadgram)
```

### Patristocrat (no word boundaries)

```bash
subsolve --patristocrat -f examples/patristocrat.txt
```

### Custom word list for segmentation

```bash
subsolve --dict examples/wordlist.txt -f examples/substitution.txt
```

`--dict` merges a custom word list (one word per line) into the built-in list used for output segmentation.

### Clues (partial known mapping)

```bash
subsolve --clues "E=T,G=R" -f examples/substitution.txt
```

Separators: `=`, `->`, `:`. Multiple clues: comma or semicolon. The mapping above is a syntax example; use clues you actually know.

## Algorithm

**Substitution mode:**

1. Load embedded English quadgram frequency table (389K entries, log-probabilities)
2. Random restart hill climbing — each restart shuffles the key, then iteratively swaps letter pairs to maximize quadgram log-probability
3. Output: Viterbi DP word segmentation (word-frequency cost) → best + 2 alternatives

**Vigenere mode:**

1. Kasiski examination + Index of Coincidence → candidate key lengths
2. χ² minimization per column → initial shift values
3. Quadgram scoring across full decrypted text
4. Hill-climbing refinement per key position (3 rounds max)
5. Output: lowercase, Viterbi DP word segmentation (word-frequency cost)

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

- **Kasiski, F. W.** (1863). _Die Geheimschriften und die Dechiffrir-Kunst_. — original publication of the Kasiski examination method for determining Vigenere key length.
- **Friedman, W. F.** (1922). _The Index of Coincidence and Its Applications in Cryptography_. — mathematical foundation for IC-based cipher analysis.
- **Gaines, H. F.** (1939). _Cryptanalysis: A Study of Ciphers and Their Solution_. — classical reference covering substitution, Vigenere, and advanced techniques. Still the go-to book for pencil-and-paper cryptanalysis.

## License

GPLv3 — see [LICENSE](LICENSE).
