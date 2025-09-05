# ib-pinyin-ahk2
`ib-pinyin` binding for [AutoHotkey](https://www.autohotkey.com/) v2.

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

[下载](https://github.com/Chaoses-Ib/ib-matcher/releases)

## Usage
[example.ahk](example.ahk):
```ahk
; Use IbPinyin32 for 32-bit AutoHotkey
; #Include <IbPinyin32>
#Include <IbPinyin>

IsMatch := IbPinyin_Match("pysousuoeve", "拼音搜索Everything")
MsgBox(IsMatch)

; 指定拼音编码
; IbPinyin_Unicode
; IbPinyin_Ascii
; IbPinyin_AsciiTone
; IbPinyin_AsciiFirstLetter
; IbPinyin_DiletterAbc
; IbPinyin_DiletterJiajia
; IbPinyin_DiletterMicrosoft
; IbPinyin_DiletterThunisoft
; IbPinyin_DiletterXiaohe
; IbPinyin_DiletterZrm
IsMatch := IbPinyin_Match("pysousuoeve", "拼音搜索Everything", IbPinyin_AsciiFirstLetter | IbPinyin_Ascii)
MsgBox(IsMatch)

; 获取匹配范围
text := "拼音搜索Everything"
IsMatch := IbPinyin_Match("pysousuoeve", text, IbPinyin_AsciiFirstLetter | IbPinyin_Ascii, &start, &end)
MsgBox(IsMatch ": " start ", " end ", " SubStr(text, start, end - start))


; 中文 API
是否匹配 := 拼音_匹配("pysousuoeve", "拼音搜索Everything")
MsgBox(是否匹配)

; 指定拼音编码
; 拼音_简拼
; 拼音_全拼
; 拼音_带声调全拼
; 拼音_Unicode
; 拼音_智能ABC双拼
; 拼音_拼音加加双拼
; 拼音_微软双拼
; 拼音_华宇双拼
; 拼音_紫光双拼
; 拼音_小鹤双拼
; 拼音_自然码双拼
是否匹配 := 拼音_匹配("pysousuoeve", "拼音搜索Everything", 拼音_简拼 | 拼音_全拼)
MsgBox(是否匹配)

; 获取匹配范围
文本 := "拼音搜索Everything"
是否匹配 := 拼音_匹配("pysousuoeve", 文本, 拼音_简拼 | 拼音_全拼, &开始位置, &结束位置)
MsgBox(是否匹配 ": " 开始位置 ", " 结束位置 ", " SubStr(文本, 开始位置, 结束位置 - 开始位置))
```

32 位相比 64 位的 DLL 体积小 0.3 MiB（1.5 → 1.2 MiB），进程总内存占用少 0.2 MiB（2.16 → 1.93 MiB）。

## Build
```pwsh
.\bindings\ahk2\build.ps1
```
`IbPinyin32.ahk` will be generated.