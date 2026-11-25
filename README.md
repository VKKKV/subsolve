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
- `--upper` / `--lower` — case conversion
- `--no-segment` — disable automatic word segmentation

**Output features:**

- Auto-lowercase with word segmentation and word-wrap
- N-best segmentation alternatives
- Verbose mode (`-v`): sorted key-candidates table with `← best` marker

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

### Solve a substitution cipher

```bash
subsolve "STLN SOENWTU OTI IYWBSOBRB T AEUB TUK DGIN KEINEUCNEYU QBNSBBU LYJEUM QF TLLYSTUCB TUK LYJEUM SENO ZBRIYUTL LYJB NOEI KEINEUCNEYU TZZLEBI NY QYYVI TI SBLL TI NY WBU TUK SYWBU TUK EU NOB CTIB YA NOB UYN JBRF UGWBRYGI TGNOYRI SOY TRB NOB YQDBCNI YA NOB ZBRIYUTL TAABCNEYU EN QREUMI T CGREYGI CYUIBHGBUCB SENO EN NOBRB EI WGCO WYRB KEAABRBUCB TI NY NOBER QBIN SYRV NOTU EU NOB CTIB YA NOYIB YNOBRI SOY TRB LYJBK QF TLLYSTUCB QF CYUJBUNEYU TUK QBCTGIB EN EI ABLN NY QB NOB REMON TUK ZRYZBR NOEUM NY LYJB NOBW TUK EU NOB IBCN ATERLF LTRMB TUK FBN"
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
subsolve -f ciphertext.txt
cat ciphertext.txt | subsolve
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

### Custom word list for segmentation

```bash
subsolve --dict /usr/share/dict/words -f ciphertext.txt
```

`--dict` adds custom words to the built-in list used for output segmentation.

### Clues (partial known mapping)

```bash
subsolve --clues "E=T,G=R" -f ciphertext.txt
```

Separators: `=`, `->`, `:`. Multiple clues: comma or semicolon.

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
