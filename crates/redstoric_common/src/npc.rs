//! Dữ liệu thuần của dân làng (NPC): tên người, tên làng, nghề, lời thoại và hàng hoá.
//! Mọi thứ sinh từ hạt giống (`seed`) nên client và server luôn cho cùng kết quả.

use crate::structures::hash2;

/// Nghề của dân làng. Quyết định tên hiển thị, lời thoại và hàng bán.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Profession {
    Farmer,
    Blacksmith,
    Alchemist,
    Merchant,
    Guard,
    Innkeeper,
    Hunter,
    Cook,
}

/// Một món hàng trong cửa hàng của NPC.
#[derive(Clone, Copy, Debug)]
pub struct Good {
    pub name: &'static str,
    pub price: u32,
}

impl Profession {
    pub const ALL: [Profession; 8] = [
        Profession::Farmer,
        Profession::Blacksmith,
        Profession::Alchemist,
        Profession::Merchant,
        Profession::Guard,
        Profession::Innkeeper,
        Profession::Hunter,
        Profession::Cook,
    ];

    pub fn from_index(i: u8) -> Self {
        Self::ALL[i as usize % Self::ALL.len()]
    }

    pub fn index(self) -> u8 {
        Self::ALL.iter().position(|p| *p == self).unwrap_or(0) as u8
    }

    pub fn label(self) -> &'static str {
        match self {
            Profession::Farmer => "Farmer",
            Profession::Blacksmith => "Blacksmith",
            Profession::Alchemist => "Alchemist",
            Profession::Merchant => "Merchant",
            Profession::Guard => "Guard",
            Profession::Innkeeper => "Innkeeper",
            Profession::Hunter => "Hunter",
            Profession::Cook => "Cook",
        }
    }

    /// Các câu thoại, lần lượt hiện ra mỗi khi bấm E.
    pub fn lines(self) -> &'static [&'static str] {
        match self {
            Profession::Farmer => &[
                "Good day, traveller! The harvest has been kind to us this season.",
                "Mind the fields, the wheat is almost ready. The scarecrow keeps most crows away.",
                "If you slay that beast in the arena, the whole village will sleep easier.",
            ],
            Profession::Blacksmith => &[
                "Welcome to my forge. Every blade here was hammered by my own hands.",
                "A sharp sword and a steady arm - that is all you need against the arena boss.",
                "Come back when you have ore. I always have work for the anvil.",
            ],
            Profession::Alchemist => &[
                "Ah, a visitor! Careful with the bottles, some of them bite.",
                "My potions mend wounds in a blink. Do you want to try one?",
                "The herbs near the forest edge are the finest. Bring me some and we will talk.",
            ],
            Profession::Merchant => &[
                "Fine goods at fair prices! Well, mostly fair.",
                "I travel between villages. The roads are quiet, but the arenas are not.",
                "Coins talk louder than swords. Take a look at my wares.",
            ],
            Profession::Guard => &[
                "Halt! ...Oh, it is only you. Carry on.",
                "I watch the road day and night. Nothing gets past me, except sleep.",
                "The beast in the old arena grows restless. Do not go there unprepared.",
            ],
            Profession::Innkeeper => &[
                "Welcome, welcome! A warm bed and a hot meal await you.",
                "Travellers bring the best stories. Got any to share?",
                "The cellar is full and the fire is lit. Stay as long as you like.",
            ],
            Profession::Hunter => &[
                "Quiet steps, quick eyes - that is how you live out here.",
                "The wolves are bold this year. Keep your bow close.",
                "Tracks lead to the arena at night. I do not follow them anymore.",
            ],
            Profession::Cook => &[
                "Something smells good, does it not? Stew of the day, fresh from the pot.",
                "A full belly makes a brave heart. Eat before you fight.",
                "The secret is patience. And a pinch of something I will not tell you.",
            ],
        }
    }

    /// Hàng bán (tối đa 4 món, ứng với phím 1-4 trong giao diện giao dịch).
    pub fn goods(self) -> &'static [Good] {
        match self {
            Profession::Farmer => &[
                Good { name: "Wheat bundle", price: 4 },
                Good { name: "Carrot", price: 2 },
                Good { name: "Pumpkin", price: 6 },
                Good { name: "Hay bale", price: 3 },
            ],
            Profession::Blacksmith => &[
                Good { name: "Iron sword", price: 60 },
                Good { name: "Iron shield", price: 45 },
                Good { name: "Whetstone", price: 8 },
                Good { name: "Iron ingot", price: 15 },
            ],
            Profession::Alchemist => &[
                Good { name: "Healing potion", price: 25 },
                Good { name: "Stamina potion", price: 20 },
                Good { name: "Antidote", price: 12 },
                Good { name: "Dried herbs", price: 5 },
            ],
            Profession::Merchant => &[
                Good { name: "Rope", price: 6 },
                Good { name: "Lantern", price: 18 },
                Good { name: "Map scrap", price: 30 },
                Good { name: "Traveller's cloak", price: 40 },
            ],
            Profession::Guard => &[
                Good { name: "Wooden shield", price: 20 },
                Good { name: "Spear", price: 35 },
                Good { name: "Torch", price: 3 },
                Good { name: "Whistle", price: 5 },
            ],
            Profession::Innkeeper => &[
                Good { name: "Hot meal", price: 8 },
                Good { name: "Ale", price: 5 },
                Good { name: "Night's rest", price: 15 },
                Good { name: "Travel bread", price: 4 },
            ],
            Profession::Hunter => &[
                Good { name: "Arrow bundle", price: 10 },
                Good { name: "Short bow", price: 50 },
                Good { name: "Leather scrap", price: 7 },
                Good { name: "Smoked meat", price: 9 },
            ],
            Profession::Cook => &[
                Good { name: "Stew", price: 10 },
                Good { name: "Fresh bread", price: 4 },
                Good { name: "Apple pie", price: 12 },
                Good { name: "Spice jar", price: 14 },
            ],
        }
    }
}

const SYL_A: [&str; 16] = [
    "A", "Al", "Ber", "Cal", "Dor", "El", "Fen", "Gar", "Hal", "Is", "Jor", "Kel", "Lor", "Mar", "Nor", "Os",
];
const SYL_B: [&str; 14] = [
    "a", "e", "i", "o", "u", "ar", "en", "il", "or", "an", "el", "in", "ad", "ur",
];
const SYL_C: [&str; 14] = [
    "m", "n", "r", "th", "s", "l", "d", "k", "b", "v", "rim", "heim", "wick", "ford",
];

/// Tên làng, ví dụ "Abeim", "Dorwick".
pub fn village_name(seed: u32) -> String {
    let a = SYL_A[hash2(seed as i32, 1, 11) as usize % SYL_A.len()];
    let b = SYL_B[hash2(seed as i32, 2, 12) as usize % SYL_B.len()];
    let c = SYL_C[hash2(seed as i32, 3, 13) as usize % SYL_C.len()];
    format!("{a}{b}{c}")
}

const FIRST_A: [&str; 12] = [
    "Sne", "Lo", "Bri", "Ta", "Gri", "Wil", "Pe", "Mo", "Ha", "Ju", "Ro", "Fi",
];
const FIRST_B: [&str; 12] = [
    "ph", "wly", "an", "bo", "sel", "mer", "tra", "dok", "len", "ric", "ko", "nia",
];

/// Tên người, ví dụ "Sneph", "Lowly".
pub fn person_name(seed: u32) -> String {
    let a = FIRST_A[hash2(seed as i32, 5, 21) as usize % FIRST_A.len()];
    let b = FIRST_B[hash2(seed as i32, 6, 22) as usize % FIRST_B.len()];
    format!("{a}{b}")
}
