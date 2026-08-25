pinyins = []
pinyin_combinations = []
pinyin_tables = []

def read_pinyins(f):
    while line := f.readline()[:-1]:
        lst = line.split(',')
        pinyins.append(tuple(lst[i] for i in (0,2,3)))

def read_pinyin_combinations(f):
    while line := f.readline()[:-1]:
        pinyin_combinations.append(line)

def read_pinyin_tables(f):
    while line := f.readline()[:-1]:
        rng = line[:-1].split(',')
        pinyin_tables.append((range(int(rng[0], 16), int(rng[1], 16) + 1), f.readline()[:-1]))

def output():
    pinyin_code = f'pub(super) const PINYINS: [&\'static str; {len(pinyins)}] = [\n'
    pinyin_code += ','.join(f'"{pinyin[0]}"' for pinyin in pinyins)
    pinyin_code += '];\n'

    max_comb = max(comb.count(",") for comb in pinyin_combinations) + 1  # 10
    comb_code = f'''pub(super) const PINYIN_COMBINATION_LEN: usize = {max_comb};

pub(super) static PINYIN_COMBINATIONS: [PinyinCombination; {len(pinyin_combinations)}] = [\n'''
    comb_code += ','.join(f'[{comb}{",F" * (max_comb - (comb.count(",") + 1))}]' for comb in pinyin_combinations)
    comb_code += '];\n'

    range_code = f'pub(super) const PINYIN_RANGE_TABLES: [PinyinRangeTable; {len(pinyin_tables)}] = [\n'
    range_code += ',\n'.join(f'PinyinRangeTable::new(0x{ table[0].start :X}..=0x{ table[0].stop - 1 :X}, &[{ table[1].replace(",65535", ",F") }])' for table in pinyin_tables)
    range_code += '\n];'

    return f'''#![cfg_attr(rustfmt, rustfmt_skip)]

use super::{{PinyinCombination, PinyinRangeTable}};

const F: u16 = u16::MAX;

{pinyin_code}
{comb_code}
{range_code}
'''

with open('data/pinyin_compact.txt', encoding='utf-8') as f:
    if f.readline() == 'pinyins:\n':
        read_pinyins(f)
    if f.readline() == 'pinyin_combinations:\n':
        read_pinyin_combinations(f)
    if f.readline() == 'pinyin_tables:\n':
        read_pinyin_tables(f)
    with open('src/data.rs', 'w', encoding='utf-8') as f:
        f.write(output())