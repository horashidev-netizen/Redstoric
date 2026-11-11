//! Trang phục dân làng: mỗi nghề mặc bộ giáp/áo của Veloren (`assets/placeholders/glb/armor/<bộ>/`) thay cho
//! bộ thân mặc định, nên nhìn là phân biệt được thợ rèn, đầu bếp, giáo sĩ, quý tộc...
//!
//! Chỉ thay các mảnh ngực / quần / thắt lưng / giày (mảnh nào bộ có). Danh sách mảnh khai báo sẵn
//! thay vì quét thư mục để chạy được trên Android (APK không quét được thư mục).

use redstoric_common::npc::Profession;

pub struct Outfit {
    pub dir: &'static str,
    pub chest: bool,
    pub pants: bool,
    pub belt: bool,
    pub foot: bool,
}

const fn o(dir: &'static str, chest: bool, pants: bool, belt: bool, foot: bool) -> Outfit {
    Outfit { dir, chest, pants, belt, foot }
}

/// `CharacterAppearance::outfit == k` (k >= 1) dùng `OUTFITS[k - 1]`; 0 là bộ thân mặc định.
pub const OUTFITS: &[Outfit] = &[
    o("rugged", true, true, false, false),             // 1
    o("leather_blue", true, true, false, false),       // 2
    o("blacksmith", true, true, true, false),          // 3
    o("savage", true, true, true, true),               // 4
    o("cloth_purple", true, true, true, true),         // 5
    o("warlock", true, true, true, true),              // 6
    o("merchant", true, true, true, true),             // 7
    o("cloth_blue", true, true, true, true),           // 8
    o("warlord", true, true, true, true),              // 9
    o("leather_plate", true, true, true, true),        // 10
    o("tarasque", true, true, true, true),             // 11
    o("cloth_green", true, true, true, true),          // 12
    o("twigsleaves", true, true, true, true),          // 13
    o("boreal", true, true, true, true),               // 14
    o("ferocious", true, true, true, true),            // 15
    o("chef", true, true, true, false),                // 16
    o("miner", true, true, false, false),              // 17
    o("velorite_battlemage", true, true, true, true),  // 18
    o("cardinal", true, true, true, true),             // 19
    o("twigsflowers", true, true, true, true),         // 20
    o("brinestone", true, true, true, true),           // 21
];

/// Các bộ (chỉ số 1-based trong `OUTFITS`) mỗi nghề có thể mặc.
fn options(job: Profession) -> &'static [usize] {
    match job {
        Profession::Farmer => &[1, 2],
        Profession::Blacksmith => &[3, 4],
        Profession::Alchemist => &[5, 6],
        Profession::Merchant => &[7, 8],
        Profession::Guard => &[9, 10, 11],
        Profession::Innkeeper => &[12, 1],
        Profession::Hunter => &[13, 14, 15],
        Profession::Cook => &[16],
        Profession::Miner => &[17, 1],
        Profession::Fisherman => &[2, 8],
        Profession::Scholar => &[18, 5],
        Profession::Priest => &[19, 12],
        Profession::Bard => &[20, 12, 21],
        Profession::Noble => &[21, 19],
    }
}

/// Bộ trang phục (1-based) của một dân làng; ổn định theo `seed`.
pub fn outfit_for(job: Profession, seed: u32) -> usize {
    let opts = options(job);
    opts[(seed.wrapping_mul(2654435761) >> 7) as usize % opts.len()]
}
