# Implement wordninja-style frequency-based Viterbi segmentation

## Background
Current segmentation uses `len²` scoring — all valid dictionary words get score based only on length. With a 370K-word dictionary, obscure 2-3 letter words compete equally with common words.

wordninja (keredson/wordninja, MIT) uses a frequency-ordered word list and Zipf's law based cost function: `cost(word) = ln((rank+1) * ln(N))` where rank is the word's 0-based position in the frequency-sorted list. This naturally prefers common words.

The word list has already been replaced: `data/words.txt` is now wordninja's 126K-word frequency-ordered list (one word per line, most frequent first).

## Changes

### `src/dict.rs`

1. **Dictionary struct**: Change from `HashSet<String>` to store word→cost mapping.
   Use `HashMap<String, f64>` where value is the pre-computed Zipf cost.

2. **`from_str()`**: Compute Zipf costs during construction:
   ```rust
   let total = word_count as f64;
   let log_total = total.ln(); // ln(N)
   for (i, word) in words.iter().enumerate() {
       let cost = ((i + 1) as f64 * log_total).ln(); // ln((rank+1) * ln(N))
       map.insert(word, cost);
   }
   ```

3. **`contains()`**: Keep as `self.words.contains_key(word)`.

4. **`segment_nbest()` scoring**: Replace `len²` with frequency-based score:
   ```rust
   let word_score = if let Some(cost) = self.words.get(candidate) {
       -cost  // lower cost = more common → higher score
   } else if candidate.len() == 1 {
       -20.0  // penalty for unknown single-letter fallback
   } else {
       // Unknown multi-letter — should not happen since we only try dict words
       -999.0
   };
   ```
   Note: `cost` is always positive (ln of something >= 1). `-cost` is negative. Since DP maximizes score, more common words (lower cost, less negative) are preferred.

5. **`score_text()`**: No changes needed — it only checks membership, not cost.

6. **Tests**: Update `test_segment_nbest`. With Zipf costs, "the quick brown fox" should still be top-ranked (common words like "the","quick","brown","fox" come BEFORE "th","equ","ick","bro","wn","fo" in the test dict). The test should assert:
   - 3 results returned
   - First result is ["the","quick","brown","fox"]
   - (Remove assertions about specific alt contents — just check count and first)

### `README.md` + `README.zh-CN.md`

Add to References section:

**Data sources:**
- **words.txt** (126K words, frequency-ordered) — from [wordninja](https://github.com/keredson/wordninja) (MIT), based on English Wikipedia unigram frequencies. Each word's position in the list determines its cost via Zipf's law: `cost = ln((rank+1) × ln(N))`. Used for Viterbi word segmentation.

**Projects:**
- **[wordninja](https://github.com/keredson/wordninja)** (keredson, MIT) — probabilistic word splitting using Wikipedia unigram frequencies and Zipf's law. The reference implementation for our Viterbi segmentation cost function.

## Verify

```bash
cargo test && cargo clippy && cargo build --release
./target/release/subsolve "DSNSM YBGVS ENQGW QNBUS KCJDQ ENQIS QGWUJ QJSVL QCNQG WANBM EDJTS JDSAS SJVSX NBTQE VQUUZ QUSCG KDCZD CJKQU SGZVB USWCJ KQUQA SQMJC XMVUZ QNQAQ SMUQG WQJJD QJJCT SMGFG BKGJB GQJMN QVCUJ UBXZB MNUSQ ENSQJ YNCPS CGQUZ CSGJC XCZYB CGJBX ICSKJ DSNSK SNSJK BNBMG WAVQZ FUYBJ UGSQN BGSSO JNSTC JLBXJ DSAQZ FQGWQ VBGEB GSGSQ NJDSB JDSNJ DSUZQ VSUKS NSSOZ SSWCG EVLDQ NWQGW EVBUU LKCJD QVVJD SQYYS QNQGZ SBXAM NGCUD SWEBV WJDSK SCEDJ BXJDS CGUSZ JKQUI SNLNS TQNFQ AVSQG WJQFC GEQVV JDCGE UCGJB ZBGUC WSNQJ CBGCZ BMVWD QNWVL AVQTS RMYCJ SNXBN DCUBY CGCBG NSUYS ZJCGE CJ"
```

The segmentation should show "with a" not "wit ha", "air and" not "ai rand", "at that time" not "att hatti me".
