use std::{io::BufReader, mem::MaybeUninit, sync::LazyLock};

use epserde::prelude::*;
use ptr_hash::{PtrHash, PtrHashParams};

type HashMap = PtrHash<u32, ptr_hash::bucket_fn::Linear>;

static MPHF: LazyLock<(HashMap, Vec<(char, char)>)> = LazyLock::new(fold_init);

const REMAP: bool = false;

include!("../../../data/case-folding-simple-chars.rs");

pub fn fold_init() -> (HashMap, Vec<(char, char)>) {
    static MPHF_B: &[u8] = include_bytes!("ptr_hash.mphf");
    let mut reader = BufReader::new(MPHF_B);
    // let mphf = HashMap::deserialize_full(&mut reader).unwrap();

    let keys = CASE_FOLDING_SIMPLE
        .iter()
        .map(|(a, _)| *a as u32)
        .collect::<Vec<_>>();
    let mphf = HashMap::new(
        &keys,
        PtrHashParams {
            single_part: true,
            remap: REMAP,
            ..PtrHashParams::default_fast()
        },
    );

    let mut v = vec![('\0', '\0'); mphf.max_index()];
    for (a, b) in CASE_FOLDING_SIMPLE {
        let i = if REMAP {
            mphf.index(&(*a as u32))
        } else {
            mphf.index_no_remap(&(*a as u32))
        };
        if !REMAP && i >= v.len() {
            v.resize(i + 1, ('\0', '\0'));
        }
        v[i] = (*a, *b);
    }

    // unsafe {
    //     MPHF = Some(mphf);
    // }
    (mphf, v)
}

pub fn fold(c: char) -> char {
    // // This struct is generic in Bytes to admit unsizing coercions.
    // #[repr(C)] // guarantee 'bytes' comes after '_align'
    // struct AlignedTo<Align, Bytes: ?Sized> {
    //     _align: [Align; 0],
    //     bytes: Bytes,
    // }

    // // dummy static used to create aligned data
    // static ALIGNED: &'static AlignedTo<u128, [u8]> = &AlignedTo {
    //     _align: [],
    //     bytes: *include_bytes!("ptr_hash.mphf"),
    // };

    // static MPHF: &'static [u8] = &ALIGNED.bytes;

    // // static MPHF: &[u8] = include_bytes!("ptr_hash.mphf");
    // let mphf = PtrHash::<u32>::deserialize_eps(MPHF).unwrap();
    // let mphf = unsafe { MPHF.as_ref().unwrap_unchecked() };
    // println!("fold_phf");

    // static FOLD: &[(char, char)] = include!("ptr_hash.in.rs");

    println!("fold_phf");

    // Long data dependency + memory barrier
    // TODO: Remove Once
    let (mphf, v) = &*MPHF;
    // TODO: FxHash
    let i = if REMAP {
        mphf.index_single_part(&(c as u32))
    } else {
        // mov     r8, cs:qword_1403561B8
        // mov     ecx, esi
        // mov     r9, 517CC1B727220A95h
        // imul    rcx, r9
        // xor     rcx, r8
        // mov     rax, rcx
        // mul     cs:qword_140356198
        // mov     rax, cs:qword_1403560F0
        // movzx   eax, byte ptr [rax+rdx]
        // xor     rax, r8
        // imul    rax, r9
        // xor     rax, rcx
        // imul    rax, cs:qword_1403561B0
        // mul     cs:qword_1403561A8
        // mov     rax, cs:qword_1403561C8
        // cmp     [rax+rdx*8], esi
        // jnz     short loc_14019F8E9
        mphf.index_single_part_no_remap(&(c as u32))
    };
    // TODO: Avoid access in miss case
    let (a, b) = unsafe { v.get_unchecked(i) };
    if *a == c {
        *b
    } else {
        // Miss
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold() {
        assert_eq!(fold('A'), 'a');
        assert_eq!(fold('!'), '!');
        assert_eq!(fold('Σ'), 'σ');
        assert_eq!(fold('う'), 'う');
    }
}

/// ucd-generate case-folding-simple ucd-16.0.0 --chars > case-folding-simple-chars.rs
// #[cfg(all(not(feature = "doc"), feature = "_test_data"))]
#[cfg(test)]
mod codegen {
    use super::*;

    use std::{fmt::Write, fs};

    include!("../../../data/case-folding-simple-chars.rs");

    #[test]
    fn codegen() {
        let keys = CASE_FOLDING_SIMPLE
            .iter()
            .map(|(a, _)| *a as u32)
            .collect::<Vec<_>>();
        dbg!(keys.len(), keys.iter().max());
        let mphf = HashMap::new(
            &keys,
            PtrHashParams {
                single_part: true,
                remap: REMAP,
                ..PtrHashParams::default_fast()
            },
        );

        let mut file = fs::File::create("src/case/fold/ptr_hash.mphf").unwrap();
        // mphf.serialize(&mut file).unwrap();

        let mut v = vec![('\0', '\0'); mphf.max_index()];
        for (a, b) in CASE_FOLDING_SIMPLE {
            let i = if REMAP {
                mphf.index(&(*a as u32))
            } else {
                mphf.index_no_remap(&(*a as u32))
            };
            if !REMAP && i >= v.len() {
                v.resize(i + 1, ('\0', '\0'));
            }
            v[i] = (*a, *b);
        }
        dbg!(v.len());

        let mut s = String::new();
        write!(s, "&[\n").unwrap();
        let mut range = 0;
        for (a, b) in v {
            write!(s, "({a:?},{b:?}),").unwrap();

            // // Natural align
            // if a as u32 / 10 != range {
            //     range = a as u32 / 10;
            //     s.push('\n');
            // }
        }
        write!(s, "\n]").unwrap();
        fs::write("src/case/fold/ptr_hash.in.rs", s).unwrap();
    }
}
