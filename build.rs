use std::path::PathBuf;
use std::{env, fs};

use allsorts::binary::read::ReadScope;
use allsorts::font::{Font, MatchingPresentation};
use allsorts::font_data::FontData;
use allsorts::subset::{CmapTarget, SubsetProfile, subset};

mod glyphs {
    include!("src/icon/glyphs.rs");
}

fn main() {
    println!("cargo::rerun-if-changed=src/icon/glyphs.rs");

    let scope = ReadScope::new(lucide_icons::LUCIDE_FONT_BYTES);
    let data = scope
        .read::<FontData<'_>>()
        .expect("the Lucide font is readable");
    let mut font = Font::new(data.table_provider(0).expect("the Lucide font has a face"))
        .expect("the Lucide font is valid");

    let glyph_ids: Vec<u16> = std::iter::once(0)
        .chain(glyphs::ALL.iter().map(|icon| {
            let (glyph, _) =
                font.lookup_glyph_index(char::from(*icon), MatchingPresentation::NotRequired, None);
            assert_ne!(glyph, 0, "{icon:?} is missing from the Lucide font");
            glyph
        }))
        .collect();

    let font_subset = subset(
        &data.table_provider(0).expect("the Lucide font has a face"),
        &glyph_ids,
        &SubsetProfile::Minimal,
        CmapTarget::Unicode,
    )
    .expect("the Lucide font can be subset");

    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("lucide.ttf");
    fs::write(out, font_subset).expect("the subset font can be written");
}
