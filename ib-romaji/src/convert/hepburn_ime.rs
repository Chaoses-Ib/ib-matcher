/*!
This module contains some functions for converting/matching Hepburn romajis to
its convenient IME variant on the fly.

- `n'` can be alternatively written as `nn`.
- `tch*` can be alternatively written as `cch*`.
*/

pub const APOSTROPHE_ALT: char = 'n';

const fn hepburn_ime_map() -> [u8; 128] {
    let mut map = [0; 128];

    // n apostrophe:
    // n' as nn
    map[b'\'' as usize] = APOSTROPHE_ALT as u8;

    // tch long consonants:
    // tcha,tchi,tcho,tchu
    // ta,te,to,tsu,tta,tte,tto,ttsu are also affected
    map[b't' as usize] = b'c';

    map
}

const HEPBURN_IME_MAP: [u8; 128] = hepburn_ime_map();

/// Only meant for internal use.
#[inline]
unsafe fn map_hepburn_ime_c(romaji: u8) -> u8 {
    debug_assert!(romaji.is_ascii());
    unsafe { *HEPBURN_IME_MAP.get_unchecked(romaji as usize) }
}

#[inline]
fn eq_ignore_hepburn_ime_c(s: u8, r: u8, r_next: u8) -> bool {
    s == r || s == unsafe { map_hepburn_ime_c(r) } && (r != b't' || r_next == b'c')
}

/**
## Performance
- TODO: GP SIMD

```x86asm
eq_ignore_hepburn_ime_equisized:
        dec     rcx
        je      .LBB0_7
        movzx   r8d, byte ptr [rdx]
        xor     eax, eax
        lea     rsi, [rip + .Lanon.de52c0b0168309d6c52539b41c92ff10.0]
        jmp     .LBB0_3
.LBB0_2:
        inc     rax
        mov     r8d, r9d
        cmp     rcx, rax
        je      .LBB0_7
.LBB0_3:
        movzx   r10d, byte ptr [rdi + rax]
        movzx   r9d, byte ptr [rdx + rax + 1]
        cmp     r10b, r8b
        je      .LBB0_2
        movzx   r11d, r8b
        cmp     r10b, byte ptr [r11 + rsi]
        jne     .LBB0_12
        cmp     r8b, 116
        sete    r8b
        cmp     r9b, 99
        setne   r10b
        test    r8b, r10b
        je      .LBB0_2
.LBB0_12:
        xor     eax, eax
        ret
.LBB0_7:
        movzx   esi, byte ptr [rdi + rcx]
        movzx   ecx, byte ptr [rdx + rcx]
        cmp     sil, cl
        je      .LBB0_10
        xor     eax, eax
        cmp     rcx, 116
        je      .LBB0_11
        lea     rdx, [rip + .Lanon.de52c0b0168309d6c52539b41c92ff10.0]
        cmp     sil, byte ptr [rcx + rdx]
        jne     .LBB0_11
.LBB0_10:
        mov     al, 1
.LBB0_11:
        ret

.Lanon.de52c0b0168309d6c52539b41c92ff10.0:
        .asciz  "\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000n\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000c\000\000\000\000\000\000\000\000\000\000"
```
*/
pub unsafe fn eq_ignore_hepburn_ime_equisized(s: &[u8], romaji: &[u8]) -> bool {
    debug_assert!(romaji.len() > 0);
    unsafe { core::hint::assert_unchecked(romaji.len() > 0) };
    debug_assert_eq!(s.len(), romaji.len());
    unsafe { core::hint::assert_unchecked(s.len() == romaji.len()) };

    // This was copied from std::str::eq_ignore_ascii_case().
    // TODO: Would comparing endings first be faster?
    // core::iter::zip(s, romaji).all(|(&s, &r)| eq_ignore_hepburn_ime_c(s, r))

    // core::iter::zip(s, romaji.windows(2)).all(|(&s, r)| unsafe {
    //     eq_ignore_hepburn_ime_c(s, *r.get_unchecked(0), *r.get_unchecked(1))
    // })
    let mut i = 0;
    let len = romaji.len() - 1;
    // Avoid je .LBB0_7
    // unsafe { core::hint::assert_unchecked(len > 0) };
    while i < len {
        unsafe {
            if !eq_ignore_hepburn_ime_c(
                *s.get_unchecked(i),
                *romaji.get_unchecked(i),
                *romaji.get_unchecked(i + 1),
            ) {
                return false;
            }
        }
        i += 1;
    }
    unsafe {
        if !eq_ignore_hepburn_ime_c(*s.get_unchecked(i), *romaji.get_unchecked(i), 0) {
            return false;
        }
    }
    true
}

pub fn starts_with_ignore_hepburn_ime(s: &str, romaji: &str) -> bool {
    if let Some(s) = s.get(..romaji.len()) {
        unsafe { eq_ignore_hepburn_ime_equisized(s.as_bytes(), romaji.as_bytes()) }
    } else {
        false
    }
}

pub fn romaji_starts_with_ignore_hepburn_ime(romaji: &str, s: &str) -> bool {
    if let Some(romaji) = romaji.get(..s.len()) {
        unsafe { eq_ignore_hepburn_ime_equisized(s.as_bytes(), romaji.as_bytes()) }
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with() {
        assert!(starts_with_ignore_hepburn_ime("kotchidayo", "kotchi"));
        assert!(starts_with_ignore_hepburn_ime("kocchidayo", "kotchi"));
        assert!(starts_with_ignore_hepburn_ime("ta", "ta"));
        assert!(starts_with_ignore_hepburn_ime("ca", "ta") == false);
        assert!(starts_with_ignore_hepburn_ime("ca", "t") == false);

        assert!(starts_with_ignore_hepburn_ime("n'isekaijoucho", "n'isekai"));
        assert!(starts_with_ignore_hepburn_ime("nnisekaijoucho", "n'isekai"));
    }

    #[test]
    fn romaji_starts_with() {
        assert!(romaji_starts_with_ignore_hepburn_ime(
            "kotchidayo",
            "kotchi",
        ));
        assert!(romaji_starts_with_ignore_hepburn_ime(
            "kotchidayo",
            "kocchi",
        ));
        assert!(romaji_starts_with_ignore_hepburn_ime("ta", "ta"));
        assert!(romaji_starts_with_ignore_hepburn_ime("ta", "ca") == false);
        assert!(romaji_starts_with_ignore_hepburn_ime("t", "ca") == false);

        assert!(romaji_starts_with_ignore_hepburn_ime(
            "n'isekaijoucho",
            "n'isekai",
        ));
        assert!(romaji_starts_with_ignore_hepburn_ime(
            "n'isekaijoucho",
            "nnisekai",
        ));
    }
}
