# subsolve

离线替换密码破解器，内置自动频率分析。

灵感来自 [quipqiup.com](https://quipqiup.com/)——专为 CTF 离线环境设计。

单二进制，零运行时依赖。

## 功能

| 模式 | 标志 | 说明 |
|------|------|------|
| 替换密码 (Substitution) | *(默认)* | 基于四字母组 (quadgram) 爬山算法 + 随机重启 |
| 维吉尼亚 (Vigenere) | `--vigenere` | IC/Kasiski 密钥长度检测 + 每列 χ² 拟合 |
| 无空格密码 (Patristocrat) | `--patristocrat` | 猜解替换关系 + Viterbi 分词 |
| 字典辅助 | `--dict <file>` | 引入自定义词表辅助评分（Bonus scoring） |
| 频率分析 | `--analyze` | 字母计数、IC 值、与英语分布对比 |
| 线索 (Clue) | `--clues "E=T,G=R"` | 锁定已知的密文→明文映射 |
| Top N | `-n 5` | 返回多个候选解 |

**实用工具：**
- `--scramble` — 随机打乱字母（生成替换密码）
- `--group [N]` / `--group5` / `--ungroup` — 按 N 个一组分块（默认 5）/ 去除分组
- `--upper` / `--lower` — 大小写转换

## 安装

### 源码编译

```bash
git clone https://git.k174.dev/k/subsolve.git
cd subsolve
cargo build --release
# 二进制文件在 target/release/subsolve
```

### 从 GitHub Releases 下载

从 [releases page](https://git.k174.dev/k/subsolve) 下载对应平台预编译二进制。

## 使用

### 替换密码

```bash
subsolve "WKH TXLFN EURZQ IRA"
# → THE QUICK BROWN FOX
```

### 从文件读入

```bash
subsolve -f ciphertext.txt
```

### 从 stdin 读入

```bash
echo "WKH TXLFN EURZQ IRA" | subsolve
```

### 频率分析（判断密码类型）

```bash
subsolve --analyze "WKH TXLFN EURZQ IRA"
```

输出包括字母频率、Index of Coincidence（IC），以及怀疑是 Vigenere 时的提示。

### Vigenere 破解

```bash
subsolve --vigenere "JGRZQ OQHCV JGGZQ"
```

指定密钥长度：

```bash
subsolve --vigenere --key-len 5 "JGRZQ OQHCV JGGZQ"
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
1. 加载 389K 条英语四字母组 (quadgram) 频率表
2. 随机重启爬山法——每次重启随机初始化密钥，然后迭代交换字母对，最大化四字母组对数概率
3. 启用 `--dict` 时，评分 = 四字母组分数 + 字典覆盖度 bonus
4. Patristocrat 模式在解密后使用 Viterbi 动态规划插入词边界

**Vigenere 模式：**
1. Kasiski 检验 + IC → 候选密钥长度
2. 每列 χ² 最小化 → 初始偏移量
3. 用四字母组模型对全文解密结果评分

## 许可

GPLv3 —— 见 [LICENSE](LICENSE)。
