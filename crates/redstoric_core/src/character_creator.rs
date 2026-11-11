//! Tạo & tuỳ chỉnh nhân vật từ các mảnh model trong `assets/placeholders/glb/figure`.
//!
//! * Chủng tộc: human, elf, dwarf, orc, danari, draugr. Giới tính: nam / nữ.
//! * Các mảnh tuỳ chọn: tóc, râu, mắt, phụ kiện (sừng, khuyên tai, băng trán...).
//! * Danh sách mảnh được quét tự động từ thư mục asset khi khởi động, nên thêm file `.glb`
//!   mới là menu tự có thêm lựa chọn.
//! * Mở / đóng menu bằng **F7**. Lựa chọn được lưu vào `character_appearance.json`.
//!
//! Nhân vật được dựng thành một bộ khung người (ngực, đầu, tay, chân) dùng lại các component
//! `Bone*` của `biped.rs`, nên hoạt ảnh chạy / đánh hiện có vẫn hoạt động.
#![allow(deprecated)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use serde::{Deserialize, Serialize};

use crate::biped::*;
use redstoric_common::paths::data_path;
use crate::{Character, WeaponHand};

const APPEARANCE_PATH: &str = "character_appearance.json";
/// Danh mục mảnh nhân vật dựng sẵn (dùng khi không quét được thư mục asset, ví dụ trên Android).
const EMBEDDED_CATALOG: &str = include_str!("../../../assets_manifest/figure_catalog.json");
const FIGURE_DIR: &str = "placeholders/glb/figure";

/// Tỉ lệ voxel -> đơn vị thế giới. Nhân vật cao khoảng 19 voxel => ~1.6 đơn vị.
pub const MODEL_SCALE: f32 = 0.085;

// ---------------------------------------------------------------------------------------------
// Dữ liệu
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Species {
    Human,
    Elf,
    Dwarf,
    Orc,
    Danari,
    Draugr,
}

impl Species {
    pub const ALL: [Species; 6] = [
        Species::Human,
        Species::Elf,
        Species::Dwarf,
        Species::Orc,
        Species::Danari,
        Species::Draugr,
    ];

    fn dir(self) -> &'static str {
        match self {
            Species::Human => "human",
            Species::Elf => "elf",
            Species::Dwarf => "dwarf",
            Species::Orc => "orc",
            Species::Danari => "danari",
            Species::Draugr => "draugr",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Species::Human => "Human",
            Species::Elf => "Elf",
            Species::Dwarf => "Dwarf",
            Species::Orc => "Orc",
            Species::Danari => "Danari",
            Species::Draugr => "Draugr",
        }
    }
}

/// Lựa chọn ngoại hình hiện tại. Với tóc / râu / phụ kiện: `0` = không có, `k` = mảnh thứ `k-1`.
/// Với mắt: chỉ số trực tiếp trong danh sách.
#[derive(Resource, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CharacterAppearance {
    pub species: Species,
    pub female: bool,
    pub hair: usize,
    pub beard: usize,
    pub eyes: usize,
    pub accessory: usize,
    /// Chức nghiệp (chỉ số trong `weapons::WEAPONS`), chọn một lần khi vào game
    pub class: usize,
    /// Đã xác nhận chức nghiệp (khoá, không đổi nữa)
    pub class_chosen: bool,
    /// Bộ trang phục (`outfits::OUTFITS`, 1-based); 0 = bộ thân mặc định. Dùng cho dân làng.
    pub outfit: usize,
    /// Màu nhân lên các mảnh (model Veloren chỉ chứa màu placeholder xám/trắng). `[1,1,1]` = giữ nguyên.
    pub hair_tint: [f32; 3],
    pub skin_tint: [f32; 3],
    pub cloth_tint: [f32; 3],
    /// Vũ khí của dân làng (đường dẫn trong `placeholders/glb/`), cất đi cho tới khi nổi giận.
    pub npc_weapon: Option<String>,
}

impl Default for CharacterAppearance {
    fn default() -> Self {
        Self {
            species: Species::Human,
            female: false,
            hair: 1,
            beard: 0,
            eyes: 0,
            accessory: 0,
            class: 0,
            class_chosen: false,
            outfit: 0,
            hair_tint: [1.0; 3],
            skin_tint: [1.0; 3],
            cloth_tint: [1.0; 3],
            npc_weapon: None,
        }
    }
}

fn load_appearance() -> CharacterAppearance {
    std::fs::read_to_string(data_path(APPEARANCE_PATH))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_appearance(appearance: &CharacterAppearance) {
    if let Ok(json) = serde_json::to_string_pretty(appearance) {
        let _ = std::fs::write(data_path(APPEARANCE_PATH), json);
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
struct PartLists {
    hair: Vec<String>,
    beard: Vec<String>,
    eyes: Vec<String>,
    accessory: Vec<String>,
}

/// Danh sách mảnh có sẵn (đường dẫn tương đối trong thư mục `figure`) theo (chủng tộc, giới tính).
#[derive(Resource, Default)]
pub struct AppearanceCatalog {
    lists: HashMap<(Species, bool), PartLists>,
}

impl AppearanceCatalog {
    /// Ngoại hình ngẫu nhiên nhưng ổn định theo `seed`, dùng cho dân làng.
    pub fn villager_appearance(&self, seed: u32) -> CharacterAppearance {
        let mut r = seed | 1;
        let mut next = |m: usize| -> usize {
            r ^= r << 13;
            r ^= r >> 17;
            r ^= r << 5;
            (r as usize) % m.max(1)
        };
        let species = match next(10) {
            0 => Species::Elf,
            1 => Species::Dwarf,
            _ => Species::Human,
        };
        let female = next(2) == 0;
        let lists = self.get(species, female);
        let hair = next(lists.hair.len() + 1);
        let beard = if !female && next(3) == 0 { next(lists.beard.len() + 1) } else { 0 };
        let eyes = next(lists.eyes.len());
        // Màu tóc / da / áo ngẫu nhiên (ổn định theo seed)
        const HAIR: [[f32; 3]; 10] = [
            [0.10, 0.09, 0.09], // đen
            [0.28, 0.17, 0.10], // nâu đậm
            [0.45, 0.28, 0.14], // nâu
            [0.56, 0.22, 0.12], // nâu đỏ
            [0.92, 0.78, 0.40], // vàng
            [0.82, 0.36, 0.12], // gừng
            [0.62, 0.62, 0.65], // xám
            [0.95, 0.95, 0.97], // trắng
            [0.55, 0.65, 0.95], // xanh nhạt (hơi huyền ảo)
            [0.80, 0.55, 0.85], // tím nhạt
        ];
        const SKIN: [[f32; 3]; 6] = [
            [1.00, 1.00, 1.00],
            [0.97, 0.91, 0.86],
            [0.88, 0.76, 0.62],
            [0.72, 0.55, 0.40],
            [0.55, 0.40, 0.30],
            [0.40, 0.29, 0.22],
        ];
        const CLOTH: [[f32; 3]; 8] = [
            [0.85, 0.25, 0.22], // đỏ
            [0.25, 0.45, 0.85], // xanh dương
            [0.30, 0.70, 0.35], // xanh lá
            [0.92, 0.80, 0.25], // vàng
            [0.60, 0.35, 0.75], // tím
            [0.55, 0.38, 0.22], // nâu
            [0.20, 0.65, 0.65], // xanh ngọc
            [0.95, 0.55, 0.20], // cam
        ];
        let hair_tint = HAIR[next(HAIR.len())];
        let skin_tint = SKIN[next(SKIN.len())];
        let cloth = CLOTH[next(CLOTH.len())];
        // Áo giữ phần lớn màu gốc của bộ giáp: chỉ ngả nhẹ sang màu đã chọn
        let cloth_tint = [0.45 + cloth[0] * 0.55, 0.45 + cloth[1] * 0.55, 0.45 + cloth[2] * 0.55];
        CharacterAppearance { species, female, hair, beard, eyes, accessory: 0, hair_tint, skin_tint, cloth_tint, ..Default::default() }
    }

    fn get(&self, species: Species, female: bool) -> PartLists {
        self.lists.get(&(species, female)).cloned().unwrap_or_default()
    }
}

fn natural_key(name: &str) -> (String, u32) {
    let stem = name.trim_end_matches(".glb");
    let digits: String = stem
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let number = digits.parse().unwrap_or(0);
    (stem[..stem.len() - digits.len()].to_string(), number)
}

fn list_glb(base: &Path, sub: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(base.join(sub))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".glb"))
        .collect();
    names.sort_by_key(|name| natural_key(name));
    names
}

fn find_figure_dir() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("assets").join(FIGURE_DIR),
        PathBuf::from("../../assets").join(FIGURE_DIR),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets").join(FIGURE_DIR),
    ];
    candidates.into_iter().find(|p| p.is_dir())
}

/// Xuất danh mục quét từ đĩa thành JSON để nhúng vào bản Android (`assets_manifest/figure_catalog.json`).
pub fn dump_catalog_json() -> String {
    let catalog = scan_dir_catalog().unwrap_or_default();
    let list: Vec<(Species, bool, PartLists)> =
        catalog.lists.iter().map(|((s, f), l)| (*s, *f, l.clone())).collect();
    serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string())
}

fn scan_catalog() -> AppearanceCatalog {
    if let Some(catalog) = scan_dir_catalog() {
        return catalog;
    }
    // Không có thư mục asset trên đĩa: dùng danh mục dựng sẵn
    let mut catalog = AppearanceCatalog::default();
    if let Ok(list) = serde_json::from_str::<Vec<(Species, bool, PartLists)>>(EMBEDDED_CATALOG) {
        for (s, f, l) in list {
            catalog.lists.insert((s, f), l);
        }
    }
    if catalog.lists.is_empty() {
        warn!("Không tìm thấy thư mục {FIGURE_DIR} hay danh mục dựng sẵn; menu tạo nhân vật sẽ trống");
    }
    catalog
}

fn scan_dir_catalog() -> Option<AppearanceCatalog> {
    let mut catalog = AppearanceCatalog::default();
    let base = find_figure_dir()?;

    for species in Species::ALL {
        let dir = species.dir();
        for female in [false, true] {
            let sex = if female { "female" } else { "male" };
            let mut lists = PartLists::default();

            // Tóc: hair/<species>/<sex>-N.glb
            for name in list_glb(&base, &format!("hair/{dir}")) {
                if name.starts_with(&format!("{sex}-")) {
                    lists.hair.push(format!("hair/{dir}/{name}"));
                }
            }
            // Râu: chỉ nam
            if !female {
                for name in list_glb(&base, &format!("beard/{dir}")) {
                    lists.beard.push(format!("beard/{dir}/{name}"));
                }
            }
            // Mắt: riêng của chủng tộc, sau đó là bộ mắt chung
            for name in list_glb(&base, &format!("eyes/{dir}")) {
                if name.starts_with(&format!("{sex}-")) || name.starts_with(&format!("{sex}_")) {
                    lists.eyes.push(format!("eyes/{dir}/{name}"));
                }
            }
            for name in list_glb(&base, "eyes/general") {
                if name.starts_with(&format!("{sex}_")) {
                    lists.eyes.push(format!("eyes/general/{name}"));
                }
            }
            // Phụ kiện: lọc theo dấu hiệu giới tính trong tên file (không có dấu hiệu => dùng chung)
            for name in list_glb(&base, &format!("accessory/{dir}")) {
                let is_female = name.contains("female");
                let is_male = name.contains("male") && !is_female;
                if (female && is_male) || (!female && is_female) {
                    continue;
                }
                lists.accessory.push(format!("accessory/{dir}/{name}"));
            }

            catalog.lists.insert((species, female), lists);
        }
    }
    Some(catalog)
}

// ---------------------------------------------------------------------------------------------
// Dựng nhân vật
// ---------------------------------------------------------------------------------------------

/// Đánh dấu entity gốc của bộ model nhân vật (bị xoá và dựng lại mỗi khi đổi ngoại hình).
#[derive(Component)]
struct HumanoidRoot;

/// Bố cục bộ khung (đơn vị voxel, gốc ở lòng bàn chân, mặt hướng -Z). Chỉnh ở đây nếu các mảnh
/// bị lệch nhau.
mod layout {
    pub const HIP_Y: f32 = 4.0; // khớp chân
    pub const FOOT_X: f32 = 2.5;
    pub const FOOT_OFFSET: (f32, f32, f32) = (0.0, -2.5, -1.0);
    pub const PANTS_Y: f32 = 4.5;
    pub const BELT_Y: f32 = 6.0;
    pub const CHEST_Y: f32 = 8.5;
    pub const HEAD_Y: f32 = 6.5; // so với ngực
    pub const SHOULDER: (f32, f32) = (8.0, 2.0); // (x, y) so với ngực
    pub const HAND_Y: f32 = -3.0; // so với vai
    pub const EYES: (f32, f32, f32) = (0.0, 0.5, -4.0); // so với đầu
    pub const BEARD: (f32, f32, f32) = (0.0, -3.0, -2.5);
    pub const ACCESSORY: (f32, f32, f32) = (0.0, 0.0, 0.0);
    pub const WEAPON: (f32, f32, f32) = (0.5, 0.5, -2.5);
}

fn rebuild_humanoid(
    mut commands: Commands,
    appearance: Res<CharacterAppearance>,
    catalog: Res<AppearanceCatalog>,
    asset_server: Res<AssetServer>,
    player: Query<Entity, With<Character>>,
    old: Query<Entity, With<HumanoidRoot>>,
) {
    if !appearance.is_changed() {
        return;
    }
    let Ok(player) = player.single() else {
        return;
    };
    for entity in &old {
        commands.entity(entity).despawn();
    }
    spawn_humanoid(&mut commands, player, &appearance, &catalog, &asset_server, true);
}

/// Dựng bộ model người dưới entity `parent`. `is_player` = true thì gắn `HumanoidRoot` (để bị
/// dựng lại khi đổi ngoại hình) và cho cầm kiếm; dân làng (false) tay không và không bị động tới.
pub fn spawn_humanoid(
    commands: &mut Commands,
    parent: Entity,
    appearance: &CharacterAppearance,
    catalog: &AppearanceCatalog,
    asset_server: &AssetServer,
    is_player: bool,
) {
    let sex = if appearance.female { "female" } else { "male" };
    let lists = catalog.get(appearance.species, appearance.female);
    let load = |rel: String| {
        asset_server.load(GltfAssetLabel::Scene(0).from_asset(format!("placeholders/glb/{rel}")))
    };

    // Trang phục: nếu có bộ giáp thì thay các mảnh mà bộ đó có, mảnh khác giữ bản mặc định
    let outfit = appearance.outfit.checked_sub(1).and_then(|i| crate::outfits::OUTFITS.get(i));
    let pick = |part: &str, has: fn(&crate::outfits::Outfit) -> bool, fallback: String| {
        load(match outfit {
            Some(o) if has(o) => format!("armor/{}/{part}.glb", o.dir),
            _ => fallback,
        })
    };
    let pants = pick("pants", |o| o.pants, format!("figure/body/pants_{sex}.glb"));
    let belt = pick("belt", |o| o.belt, format!("figure/body/belt_{sex}.glb"));
    let chest = pick("chest", |o| o.chest, format!("figure/body/chest_{sex}.glb"));
    let foot = pick("foot", |o| o.foot, "figure/body/foot.glb".to_string());
    let hand = load("figure/body/hand.glb".to_string());
    let head = load(format!("figure/head/{}/{sex}.glb", appearance.species.dir()));
    let hair = appearance.hair.checked_sub(1).and_then(|i| lists.hair.get(i)).map(|p| load(format!("figure/{p}")));
    let beard = appearance.beard.checked_sub(1).and_then(|i| lists.beard.get(i)).map(|p| load(format!("figure/{p}")));
    let eyes = lists.eyes.get(appearance.eyes).map(|p| load(format!("figure/{p}")));
    let accessory = appearance
        .accessory
        .checked_sub(1)
        .and_then(|i| lists.accessory.get(i))
        .map(|p| load(format!("figure/{p}")));
    let weapon = if is_player {
        Some(load("weapon/sword/caladbolg.glb".to_string()))
    } else {
        appearance.npc_weapon.clone().map(load)
    };
    let hair_c = Color::srgb(appearance.hair_tint[0], appearance.hair_tint[1], appearance.hair_tint[2]);
    let skin_c = Color::srgb(appearance.skin_tint[0], appearance.skin_tint[1], appearance.skin_tint[2]);
    let cloth_c = Color::srgb(appearance.cloth_tint[0], appearance.cloth_tint[1], appearance.cloth_tint[2]);
    let hair_on = appearance.hair_tint != [1.0; 3];
    let skin_on = appearance.skin_tint != [1.0; 3];
    let cloth_on = appearance.cloth_tint != [1.0; 3];

    // Hoạt ảnh dùng chung (người chơi + NPC); chèn lại khi dựng lại model để tìm lại khớp
    commands.entity(parent).insert(AnimDriver::default());
    commands.entity(parent).with_children(|root| {
        let mut body_root = root.spawn((Transform::from_scale(Vec3::splat(MODEL_SCALE)), Visibility::default(), Limb::Body));
        if is_player {
            body_root.insert(HumanoidRoot);
        }
        body_root.with_children(|body| {
            // Hông + thắt lưng (đứng yên theo thân)
            body.spawn((WorldAssetRoot(pants), Transform::from_xyz(0.0, layout::PANTS_Y, 0.0)))
                .insert_if(PartTint(cloth_c), || cloth_on);
            body.spawn((WorldAssetRoot(belt), Transform::from_xyz(0.0, layout::BELT_Y, 0.0)));

            // Chân (bàn chân là phần chân của model "chibi"), xoay quanh khớp hông
            let (fx, fy, fz) = layout::FOOT_OFFSET;
            body.spawn((
                Transform::from_xyz(-layout::FOOT_X, layout::HIP_Y, 0.0),
                Visibility::default(),
            ))
            .insert_if(BoneLegLeft, || is_player)
            .insert(Limb::LegL)
            .with_children(|leg| {
                leg.spawn((WorldAssetRoot(foot.clone()), Transform::from_xyz(fx, fy, fz)));
            });
            body.spawn((
                Transform::from_xyz(layout::FOOT_X, layout::HIP_Y, 0.0),
                Visibility::default(),
            ))
            .insert_if(BoneLegRight, || is_player)
            .insert(Limb::LegR)
            .with_children(|leg| {
                leg.spawn((WorldAssetRoot(foot), Transform::from_xyz(fx, fy, fz)));
            });

            // Ngực + đầu + tay
            body.spawn((
                Transform::from_xyz(0.0, layout::CHEST_Y, 0.0),
                Visibility::default(),
            ))
            .insert_if(BoneChest, || is_player)
            .insert(Limb::Chest)
            .with_children(|c| {
                c.spawn((WorldAssetRoot(chest), Transform::IDENTITY))
                    .insert_if(PartTint(cloth_c), || cloth_on);

                // Đầu và các mảnh gắn trên đầu
                c.spawn((
                    Transform::from_xyz(0.0, layout::HEAD_Y, 0.0),
                    Visibility::default(),
                ))
                .insert_if(BoneHead, || is_player)
                .insert(Limb::Head)
                .with_children(|h| {
                    h.spawn((WorldAssetRoot(head), Transform::IDENTITY))
                        .insert_if(PartTint(skin_c), || skin_on);
                    if let Some(scene) = hair {
                        h.spawn((WorldAssetRoot(scene), Transform::IDENTITY))
                            .insert_if(PartTint(hair_c), || hair_on);
                    }
                    if let Some(scene) = eyes {
                        let (x, y, z) = layout::EYES;
                        h.spawn((WorldAssetRoot(scene), Transform::from_xyz(x, y, z)));
                    }
                    if let Some(scene) = beard {
                        let (x, y, z) = layout::BEARD;
                        h.spawn((WorldAssetRoot(scene), Transform::from_xyz(x, y, z)))
                            .insert_if(PartTint(hair_c), || hair_on);
                    }
                    if let Some(scene) = accessory {
                        let (x, y, z) = layout::ACCESSORY;
                        h.spawn((WorldAssetRoot(scene), Transform::from_xyz(x, y, z)));
                    }
                });

                // Tay trái
                c.spawn((
                    Transform::from_xyz(-layout::SHOULDER.0, layout::SHOULDER.1, 0.0),
                    Visibility::default(),
                ))
                .insert_if(BoneShoulderLeft, || is_player)
                .insert(Limb::ShoulderL)
                .with_children(|sh| {
                    sh.spawn((
                        WorldAssetRoot(hand.clone()),
                        Transform::from_xyz(0.0, layout::HAND_Y, 0.0),
                    ))
                    .insert_if(BoneHandLeft, || is_player)
                    .insert_if(PartTint(skin_c), || skin_on);
                });

                // Tay phải cầm kiếm
                c.spawn((
                    Transform::from_xyz(layout::SHOULDER.0, layout::SHOULDER.1, 0.0),
                    Visibility::default(),
                ))
                .insert_if(BoneShoulderRight, || is_player)
                .insert(Limb::ShoulderR)
                .insert_if(NpcArmRight, || !is_player)
                .with_children(|sh| {
                    sh.spawn((
                        Transform::from_xyz(0.0, layout::HAND_Y, 0.0),
                        Visibility::default(),
                    ))
                    .insert_if(BoneHandRight, || is_player)
                    .with_children(|hand_node| {
                        hand_node.spawn((WorldAssetRoot(hand), Transform::IDENTITY))
                            .insert_if(PartTint(skin_c), || skin_on);
                        if let Some(weapon) = weapon {
                            let (x, y, z) = layout::WEAPON;
                            if is_player {
                                hand_node.spawn((
                                    WorldAssetRoot(weapon),
                                    Transform::from_xyz(x, y, z),
                                    WeaponHand,
                                ));
                            } else {
                                // Không gắn `WeaponHand`: hệ thống đổi vũ khí/hoạt ảnh của người chơi dùng nhãn đó
                                hand_node.spawn((
                                    WorldAssetRoot(weapon),
                                    Transform::from_xyz(x, y, z),
                                    Visibility::Hidden,
                                    NpcWeapon,
                                ));
                            }
                        }
                    });
                });
            });
        });
    });
}

// ---------------------------------------------------------------------------------------------
// Menu
// ---------------------------------------------------------------------------------------------

const BTN_NORMAL: Color = Color::srgb(0.20, 0.22, 0.28);
const BTN_HOVER: Color = Color::srgb(0.30, 0.34, 0.44);
const BTN_PRESSED: Color = Color::srgb(0.12, 0.14, 0.20);

/// Trạng thái menu. Hệ thống chiến đấu đọc `open` để bỏ qua click khi đang bấm trong menu.
#[derive(Resource, Default)]
pub struct CreatorMenu {
    pub open: bool,
}

#[derive(Component)]
struct CreatorRoot;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Class,
    Species,
    Sex,
    Hair,
    Beard,
    Eyes,
    Accessory,
}

#[derive(Component, Clone, Copy, PartialEq)]
enum CreatorAction {
    Step(Field, i32),
    Randomize,
    Confirm,
    Close,
}

#[derive(Component)]
struct FieldText(Field);

/// Màu nhân lên mọi mesh của scene con (model Veloren chỉ có màu placeholder; màu thật chọn lúc chạy).
#[derive(Component)]
pub struct PartTint(pub Color);

#[derive(Component)]
struct TintApplied;

/// Vũ khí trên tay dân làng (ẩn cho tới khi nổi giận).
#[derive(Component)]
pub struct NpcWeapon;

/// Vai phải của dân làng (để vung vũ khí).
#[derive(Component)]
pub struct NpcArmRight;

/// Khi scene glb của một mảnh đã dựng xong thì nhân màu `PartTint` vào bản sao riêng của từng material
/// (không sửa material dùng chung nên các NPC khác không bị đổi màu).
fn apply_part_tints(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    roots: Query<(Entity, &PartTint), Without<TintApplied>>,
    children: Query<&Children>,
    mut handles: Query<&mut MeshMaterial3d<StandardMaterial>>,
    // Material đã nhuộm dùng chung theo (material gốc, màu): các NPC cùng màu chia sẻ một material nên
    // GPU gộp được lệnh vẽ, thay vì mỗi NPC một bộ material riêng.
    mut cache: Local<HashMap<(AssetId<StandardMaterial>, [u8; 3]), Handle<StandardMaterial>>>,
) {
    for (root, tint) in &roots {
        let t = tint.0.to_linear();
        let mut stack: Vec<Entity> = vec![root];
        let mut changed = false;
        while let Some(e) = stack.pop() {
            if let Ok(mut handle) = handles.get_mut(e) {
                let key = (handle.0.id(), [(t.red * 255.0) as u8, (t.green * 255.0) as u8, (t.blue * 255.0) as u8]);
                if let Some(shared) = cache.get(&key) {
                    handle.0 = shared.clone();
                    changed = true;
                } else if let Some(mut mat) = materials.get(&handle.0).cloned() {
                    let c = mat.base_color.to_linear();
                    mat.base_color = Color::linear_rgba(c.red * t.red, c.green * t.green, c.blue * t.blue, c.alpha);
                    mat.alpha_mode = AlphaMode::Opaque;
                    mat.cull_mode = None;
                    mat.double_sided = true;
                    let shared = materials.add(mat);
                    cache.insert(key, shared.clone());
                    handle.0 = shared;
                    changed = true;
                }
            }
            if let Ok(kids) = children.get(e) {
                for kid in kids.iter() {
                    stack.push(kid.clone());
                }
            }
        }
        if changed {
            commands.entity(root).insert(TintApplied);
        }
    }
}

pub struct CharacterCreatorPlugin;

impl Plugin for CharacterCreatorPlugin {
    fn build(&self, app: &mut App) {
        let saved = load_appearance();
        // Lần đầu vào game (chưa chọn chức nghiệp): tự mở menu
        let open = !saved.class_chosen;
        app.insert_resource(saved)
            .insert_resource(scan_catalog())
            .insert_resource(CreatorMenu { open })
            .add_systems(Startup, spawn_creator_menu)
            .add_systems(
                Update,
                (toggle_creator, handle_creator_buttons, refresh_creator, rebuild_humanoid).chain(),
            )
            .add_systems(Update, apply_part_tints);
    }
}

fn menu_text(text: &str, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

macro_rules! creator_button {
    ($parent:expr, $label:expr, $action:expr) => {
        $parent
            .spawn((
                Interaction::None,
                FocusPolicy::Block,
                Node {
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(BTN_NORMAL),
                $action,
            ))
            .with_children(|button| {
                button.spawn(menu_text($label, 16.0, Color::WHITE));
            });
    };
}

fn spawn_creator_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                padding: UiRect::left(Val::Px(24.0)),
                ..default()
            },
            Visibility::Hidden,
            CreatorRoot,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        min_width: Val::Px(340.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.05, 0.06, 0.09, 0.94)),
                ))
                .with_children(|panel| {
                    panel.spawn(menu_text("Character  [F7]", 22.0, Color::WHITE));
                    panel.spawn(menu_text("Chon chuc nghiep (chi 1 lan) roi bam Confirm", 14.0, Color::srgb(1.0, 0.85, 0.4)));

                    for (name, field) in [
                        ("Class", Field::Class),
                        ("Species", Field::Species),
                        ("Sex", Field::Sex),
                        ("Hair", Field::Hair),
                        ("Beard", Field::Beard),
                        ("Eyes", Field::Eyes),
                        ("Accessory", Field::Accessory),
                    ] {
                        panel
                            .spawn(Node {
                                column_gap: Val::Px(10.0),
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|row| {
                                creator_button!(row, "<", CreatorAction::Step(field, -1));
                                row.spawn((
                                    Node {
                                        min_width: Val::Px(210.0),
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    Visibility::default(),
                                ))
                                .with_children(|label| {
                                    label
                                        .spawn(menu_text(&format!("{name}: "), 16.0, Color::WHITE))
                                        .with_children(|text| {
                                            text.spawn((
                                                TextSpan::new("..."),
                                                TextFont {
                                                    font_size: FontSize::Px(16.0),
                                                    ..default()
                                                },
                                                TextColor(Color::srgb(0.6, 0.9, 1.0)),
                                                FieldText(field),
                                            ));
                                        });
                                });
                                creator_button!(row, ">", CreatorAction::Step(field, 1));
                            });
                    }

                    panel
                        .spawn(Node {
                            column_gap: Val::Px(10.0),
                            margin: UiRect::top(Val::Px(6.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            creator_button!(row, "Randomize", CreatorAction::Randomize);
                            creator_button!(row, "Confirm", CreatorAction::Confirm);
                            creator_button!(row, "Close (F7)", CreatorAction::Close);
                        });
                });
        });
}

fn toggle_creator(keys: Res<ButtonInput<KeyCode>>, mut menu: ResMut<CreatorMenu>, appearance: Res<CharacterAppearance>) {
    // Chưa chọn chức nghiệp thì menu không đóng được
    if keys.just_pressed(KeyCode::F7) && appearance.class_chosen {
        menu.open = !menu.open;
    }
}

/// Đặt lại các mảnh tuỳ chọn khi đổi chủng tộc / giới tính (danh sách mảnh khác nhau).
fn reset_parts(app: &mut CharacterAppearance, catalog: &AppearanceCatalog) {
    let lists = catalog.get(app.species, app.female);
    app.hair = if lists.hair.is_empty() { 0 } else { 1 };
    app.beard = 0;
    app.eyes = 0;
    app.accessory = 0;
}

fn step_field(app: &mut CharacterAppearance, catalog: &AppearanceCatalog, field: Field, delta: i32) {
    let lists = catalog.get(app.species, app.female);
    let wrap = |cur: usize, n: usize| -> usize {
        if n == 0 {
            0
        } else {
            (cur as i32 + delta).rem_euclid(n as i32) as usize
        }
    };
    match field {
        Field::Class => {
            if !app.class_chosen {
                let n = crate::weapons::WEAPONS.len() as i32;
                app.class = (app.class as i32 + delta).rem_euclid(n) as usize;
            }
        }
        Field::Species => {
            let i = Species::ALL.iter().position(|s| *s == app.species).unwrap_or(0);
            let next = (i as i32 + delta).rem_euclid(Species::ALL.len() as i32) as usize;
            app.species = Species::ALL[next];
            reset_parts(app, catalog);
        }
        Field::Sex => {
            app.female = !app.female;
            reset_parts(app, catalog);
        }
        Field::Hair => app.hair = wrap(app.hair, lists.hair.len() + 1),
        Field::Beard => app.beard = wrap(app.beard, lists.beard.len() + 1),
        Field::Eyes => app.eyes = wrap(app.eyes, lists.eyes.len()),
        Field::Accessory => app.accessory = wrap(app.accessory, lists.accessory.len() + 1),
    }
}

fn randomize(app: &mut CharacterAppearance, catalog: &AppearanceCatalog) {
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(12345)
        | 1;
    let mut next = |n: usize| -> usize {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        if n == 0 { 0 } else { seed as usize % n }
    };
    app.species = Species::ALL[next(Species::ALL.len())];
    app.female = next(2) == 1;
    let lists = catalog.get(app.species, app.female);
    app.hair = next(lists.hair.len() + 1);
    app.beard = if app.female { 0 } else { next(lists.beard.len() + 1) };
    app.eyes = next(lists.eyes.len());
    app.accessory = if next(3) == 0 { next(lists.accessory.len() + 1) } else { 0 };
}

fn handle_creator_buttons(
    interactions: Query<(&Interaction, &CreatorAction), Changed<Interaction>>,
    mut menu: ResMut<CreatorMenu>,
    mut appearance: ResMut<CharacterAppearance>,
    catalog: Res<AppearanceCatalog>,
) {
    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            CreatorAction::Step(field, delta) => {
                let mut next = appearance.clone();
                step_field(&mut next, &catalog, field, delta);
                appearance.set_if_neq(next);
            }
            CreatorAction::Randomize => {
                let mut next = appearance.clone();
                randomize(&mut next, &catalog);
                appearance.set_if_neq(next);
            }
            CreatorAction::Confirm => {
                let mut next = appearance.clone();
                next.class_chosen = true;
                appearance.set_if_neq(next);
                menu.open = false;
            }
            CreatorAction::Close => {
                if appearance.class_chosen {
                    menu.open = false;
                }
            }
        }
        save_appearance(&appearance);
    }
}

fn refresh_creator(
    menu: Res<CreatorMenu>,
    appearance: Res<CharacterAppearance>,
    catalog: Res<AppearanceCatalog>,
    mut roots: Query<&mut Visibility, With<CreatorRoot>>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<CreatorAction>>,
    mut texts: Query<(&FieldText, &mut TextSpan)>,
) {
    let wanted = if menu.open { Visibility::Inherited } else { Visibility::Hidden };
    for mut visibility in &mut roots {
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
    if !menu.open {
        return;
    }

    for (interaction, mut background) in &mut buttons {
        let color = match *interaction {
            Interaction::Pressed => BTN_PRESSED,
            Interaction::Hovered => BTN_HOVER,
            Interaction::None => BTN_NORMAL,
        };
        if background.0 != color {
            background.0 = color;
        }
    }

    if menu.is_changed() || appearance.is_changed() {
        let lists = catalog.get(appearance.species, appearance.female);
        let optional = |index: usize, total: usize| {
            if index == 0 {
                "None".to_string()
            } else {
                format!("{index}/{total}")
            }
        };
        for (field, mut span) in &mut texts {
            **span = match field.0 {
                Field::Class => {
                    let name = crate::weapons::CLASS_LABELS[appearance.class.min(crate::weapons::WEAPONS.len() - 1)];
                    if appearance.class_chosen { format!("{name} (locked)") } else { name.to_string() }
                }
                Field::Species => appearance.species.label().to_string(),
                Field::Sex => (if appearance.female { "Female" } else { "Male" }).to_string(),
                Field::Hair => optional(appearance.hair, lists.hair.len()),
                Field::Beard => optional(appearance.beard, lists.beard.len()),
                Field::Eyes => {
                    if lists.eyes.is_empty() {
                        "None".to_string()
                    } else {
                        format!("{}/{}", appearance.eyes + 1, lists.eyes.len())
                    }
                }
                Field::Accessory => optional(appearance.accessory, lists.accessory.len()),
            };
        }
    }
}
