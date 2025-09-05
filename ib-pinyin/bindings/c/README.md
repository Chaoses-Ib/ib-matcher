# ib-pinyin-c
一个高性能拼音匹配库。

- 支持以下拼音编码方案：
  - 简拼（“py”）
  - 全拼（“pinyin”）
  - 带声调全拼（“pin1yin1”）
  - Unicode（“pīnyīn”）
  - 智能 ABC 双拼
  - 拼音加加双拼
  - 微软双拼
  - 华宇双拼（紫光双拼）
  - 小鹤双拼
  - 自然码双拼
- 支持多音字。
- 支持混合匹配多种拼音编码方案，默认匹配简拼和全拼。
- 默认小写字母匹配拼音或字母，大写字母只匹配字母。
- 支持 Unicode 辅助平面汉字。

## Usage
```c
#include <ib_pinyin/ib_pinyin.h>
#include <ib_pinyin/notation.h>

// UTF-8
bool is_match = ib_pinyin_is_match_u8c(u8"pysousuoeve", u8"拼音搜索Everything", PINYIN_NOTATION_ASCII_FIRST_LETTER | PINYIN_NOTATION_ASCII);

// UTF-16
bool is_match = ib_pinyin_is_match_u16c(u"pysousuoeve", u"拼音搜索Everything", PINYIN_NOTATION_ASCII_FIRST_LETTER | PINYIN_NOTATION_ASCII);

// UTF-32
bool is_match = ib_pinyin_is_match_u32c(U"pysousuoeve", U"拼音搜索Everything", PINYIN_NOTATION_ASCII_FIRST_LETTER | PINYIN_NOTATION_ASCII);
```

Find match:
```c
uint64_t match = ib_pinyin_find_match_u8c(u8"pysousuoeve", u8"拼音搜索Everything", PINYIN_NOTATION_ASCII_FIRST_LETTER | PINYIN_NOTATION_ASCII);
uint32_t start = match & 0xFFFFFFFF;
uint32_t end = match >> 32;
bool is_match = start != 0xFFFFFFFF;
```

## Build
```sh
cd ib-pinyin/bindings/c
cargo build -r
```
Then you can find `ib_pinyin_c.lib` and `ib_pinyin_c.dll` (and `ib_pinyin_c.dll.lib`) in `target/release`.

## Development
```sh
diplomat-tool c bindings/c/include/ib_pinyin -e bindings/c/src/lib.rs
```

Manually update: `notation.h`