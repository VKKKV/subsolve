# subsolve

离线密码破解器——替换密码 & Vigenere。

灵感来自 [quipqiup.com](https://quipqiup.com/)——专为 CTF 离线环境设计。
单二进制，零运行时依赖。内嵌语言模型（~7 MB）。

## 功能

| 模式 | 标志 | 说明 |
|------|------|------|
| 替换密码 (Substitution) | *(默认)* | 四字母组 (quadgram) 爬山算法 + 随机重启 |
| 维吉尼亚 (Vigenere) | `--vigenere` | Kasiski/IC 密钥长度检测 + 每列 χ² 拟合 + quadgram 精炼 |
| 无空格密码 (Patristocrat) | `--patristocrat` | 替换密码 + Viterbi 分词 |
| 字典辅助 | `--dict <file>` | 自定义词表辅助评分 |
| 频率分析 | `--analyze` | 字母计数、IC 值、英语分布对比 |
| 线索 (Clue) | `--clues "E=T,G=R"` | 锁定已知密文→明文映射 |
| Top N | `-n 5` | 返回多个候选解 |

**实用工具：**
- `--scramble` — 随机打乱字母（生成替换密码密文）
- `--group [N]` / `--group5` / `--ungroup` — 按 N 个一组分块（默认 5）/ 去除分组
- `--upper` / `--lower` — 大小写转换
- `--no-segment` — 关闭自动分词

**输出特性（v0.2+）：**
- 默认小写 + 自动分词 + 自动换行
- N-best 分词候选，每个候选显示 quadgram 分数
- Verbose 模式 (`-v`)：排序后的密钥候选表，含 `← best` 标记

## 安装

### 源码编译

```bash
git clone git@github.com:VKKKV/subsolve.git
cd subsolve
cargo build --release
# 二进制文件在 target/release/subsolve
```

### 从 GitHub Releases 下载

从 [releases page](https://github.com/VKKKV/subsolve/releases) 下载对应平台预编译二进制。

## 使用

### 替换密码

```bash
subsolve "WKH TXLFN EURZQ IRA"
```

默认输出：小写、自动分词、含备选方案。

### 从文件 / stdin 读入

```bash
subsolve -f ciphertext.txt
echo "WKH TXLFN EURZQ IRA" | subsolve
```

### 频率分析（判断密码类型）

```bash
subsolve --analyze "WKH TXLFN EURZQ IRA"
```

输出字母频率、IC 值，如果怀疑是 Vigenere 会有提示。

### Vigenere 破解

```bash
subsolve --vigenere "JGRZQ OQHCV JGGZQ"
subsolve --vigenere --key-len 5 "JGRZQ OQHCV JGGZQ"    # 已知密钥长度
subsolve --vigenere -v "JGRZQ OQHCV JGGZQ"               # verbose: 查看所有候选
```

### 无空格密码（Patristocrat）

```bash
subsolve --patristocrat "WKHQWKHTXLFNEURZQIRAMXPSVRYHUWKHODCBGRJ"
```

### 字典辅助破解

```bash
subsolve --dict /usr/share/dict/words "WKH TXLFN EURZQ IRA"
```

### 线索（已知部分映射）

```bash
subsolve --clues "E=T,G=R" "WKH TXLFN EURZQ IRA"
```

分隔符支持 `=`、`->`、`:`；多个线索用逗号或分号分隔。

## 算法

**替换密码模式：**
1. 加载内嵌英语四字母组频率表（389K 条，对数概率）
2. 随机重启爬山法——每次重启随机初始化密钥，迭代交换字母对，最大化四字母组对数概率
3. 启用 `--dict` 时：四字母组分数 + 字典覆盖度 bonus
4. 输出：Viterbi 动态规划分词 + quadgram 重排序 → 最佳解 + 2 个备选

**Vigenere 模式：**
1. Kasiski 检验 + IC → 候选密钥长度
2. 每列 χ² 最小化 → 初始偏移量
3. 用四字母组模型对全文解密结果评分
4. 逐位置爬山精炼（最多 3 轮）
5. 输出：小写、Viterbi 分词 + quadgram 重排序

## 参考

### 项目 & 工具

- **[quipqiup](https://quipqiup.com/)** (Edwin Olson) — 高速在线密码破解器。核心 UX 灵感来源：多候选带分数展示、默认小写输出。
- **[Practical Cryptography](http://practicalcryptography.com/)** (James Lyons) — 经典密码分析的综合资源。quadgram 模型、Kasiski 检测、χ² 检验、爬山算法均参考此站。
- **[dcode.fr](https://www.dcode.fr/)** — 在线密码工具箱。Vigenere IC 密钥长度检测的参考实现。
- **[CrypTool](https://www.cryptool.org/)** — 古典密码学教育软件。

### 数据来源

- **english_quadgrams.txt** (389K 条, 3.2 MB) — 英语四字母组对数概率，来自 practicalcryptography.com。编译时内嵌。
- **words.txt** (37 万词, 4.1 MB) — 英语词表，来自 [SCOWL](http://wordlist.aspell.net/) (Spell Checker Oriented Word Lists) 项目，size 70 变体。用于 Viterbi 分词。编译时内嵌。

### 经典文献

- **Kasiski, F. W.** (1863). *Die Geheimschriften und die Dechiffrir-Kunst*. — Kasiski 检验法的原始文献。
- **Friedman, W. F.** (1922). *The Index of Coincidence and Its Applications in Cryptography*. — IC 分析的数学基础。
- **Gaines, H. F.** (1939). *Cryptanalysis: A Study of Ciphers and Their Solution*. — 涵盖替换密码、Vigenere 及高级技术的经典著作，至今仍是纸笔密码分析的首选参考书。

## 许可

GPLv3 —— 见 [LICENSE](LICENSE)。
