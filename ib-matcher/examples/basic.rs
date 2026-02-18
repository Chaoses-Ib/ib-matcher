use ib_matcher::{
    matcher::{IbMatcher, PinyinMatchConfig, RomajiMatchConfig},
    pinyin::PinyinNotation,
};

fn main() {
    let matcher = IbMatcher::builder("pysousuoeve")
        .pinyin(PinyinMatchConfig::notations(
            PinyinNotation::Ascii | PinyinNotation::AsciiFirstLetter,
        ))
        .build();
    assert!(matcher.is_match("拼音搜索Everything"));

    let matcher = IbMatcher::builder("konosuba")
        .romaji(RomajiMatchConfig::default())
        .build();
    assert!(matcher.is_match("この素晴らしい世界に祝福を"));
}
