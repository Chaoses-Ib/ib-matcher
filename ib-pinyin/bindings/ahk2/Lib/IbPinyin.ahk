; IbPinyinLib
; https://github.com/Chaoses-Ib/ib-matcher

; Use IbPinyin for 64-bit AutoHotkey, IbPinyin32 for 32-bit AutoHotkey
#Requires AutoHotkey v2.0 64-bit

; #DllLoad cannot be executed conditionally
#DllLoad IbPinyin64.dll

IbPinyin_Unicode := 0x8
IbPinyin_Ascii := 0x2
IbPinyin_AsciiTone := 0x4
IbPinyin_AsciiFirstLetter := 0x1
IbPinyin_DiletterAbc := 0x10
IbPinyin_DiletterJiajia := 0x20
IbPinyin_DiletterMicrosoft := 0x40
IbPinyin_DiletterThunisoft := 0x80
IbPinyin_DiletterXiaohe := 0x100
IbPinyin_DiletterZrm := 0x200
IbPinyin_PatternPartial := 0x40000000

IbPinyin_IsMatch(pattern, haystack, notations := IbPinyin_AsciiFirstLetter | IbPinyin_Ascii)
{
    ; If DllCall's first parameter is a literal string such as "MulDiv" and the DLL containing the function is ordinarily loaded before the script starts, or has been successfully loaded with #DllLoad, the string is automatically resolved to a function address.
    return DllCall("IbPinyin64\ib_pinyin_is_match_u16", "Ptr", StrPtr(pattern), "UPtr", StrLen(pattern), "Ptr", StrPtr(haystack), "UPtr", StrLen(haystack), "UInt", notations, "Cdecl Int") & 0xFF != 0
}

IbPinyin_FindMatch(pattern, haystack, &start, &end, notations := IbPinyin_AsciiFirstLetter | IbPinyin_Ascii)
{
    u64 := DllCall("IbPinyin64\ib_pinyin_find_match_u16", "Ptr", StrPtr(pattern), "UPtr", StrLen(pattern), "Ptr", StrPtr(haystack), "UPtr", StrLen(haystack), "UInt", notations, "Cdecl UInt64")
    start := Mod((u64 & 0xFFFFFFFF) + 1, 0x100000000)
    end := Mod((u64 >>> 32) + 1, 0x100000000)
    return start != 0
}

IbPinyin_Match(pattern, haystack, notations := IbPinyin_AsciiFirstLetter | IbPinyin_Ascii, &start := 0, &end := 0)
{
    return IbPinyin_FindMatch(pattern, haystack, &start, &end, notations)
}

拼音_简拼 := 1
拼音_全拼 := 2
拼音_带声调全拼 := 4
拼音_Unicode := 8
拼音_智能ABC双拼 := 16
拼音_拼音加加双拼 := 32
拼音_微软双拼 := 64
拼音_华宇双拼 := 128
拼音_紫光双拼 := 128
拼音_小鹤双拼 := 256
拼音_自然码双拼 := 512

拼音_匹配(关键字, 文本, 拼音 := 拼音_简拼 | 拼音_全拼, &开始 := 0, &结束 := 0) {
    return IbPinyin_Match(关键字, 文本, 拼音, &开始, &结束)
}