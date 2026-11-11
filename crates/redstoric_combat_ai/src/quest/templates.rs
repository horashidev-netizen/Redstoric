#![allow(unused_imports)]
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    Hunt(Option<Species>),
    Slay,
    Courier,
    Fetch,
    Scout,
    Patrol,
}

/// Mẫu nhiệm vụ. Trong `intro`: {n} số lượng, {place} nơi đến, {boss} tên boss, {item} vật phẩm.
pub(super) struct Template {
    pub(super) kind: Kind,
    pub(super) title: &'static str,
    pub(super) item: &'static str,
    pub(super) intro: &'static str,
}

pub(super) const fn t(kind: Kind, title: &'static str, item: &'static str, intro: &'static str) -> Template {
    Template { kind, title, item, intro }
}

use Kind::*;

pub(super) const WOLF: Option<Species> = Some(Species::Wolf);

pub(super) const PIG: Option<Species> = Some(Species::Pig);

pub(super) const RABBIT: Option<Species> = Some(Species::Rabbit);

pub(super) const DEER: Option<Species> = Some(Species::Deer);

pub(super) const FARMER: &[Template] = &[
    t(Hunt(RABBIT), "Garden Pests", "", "Rabbits are eating my carrots! Catch {n} of them."),
    t(Hunt(WOLF), "Wolves at the Fence", "", "Wolves keep stalking my flock. Drive off {n} of them."),
    t(Fetch, "Spilled Harvest", "Pumpkin", "My cart tipped over out on the road. Gather {n} pumpkins for me."),
];

pub(super) const BLACKSMITH: &[Template] = &[
    t(Fetch, "Ore for the Forge", "Iron ore", "I'm out of ore. Dig up {n} chunks from the marked rocks."),
    t(Courier, "Tool Order", "", "Deliver these tools to the smithy in {place}."),
    t(Slay, "Guardian's Steel", "", "The {boss} of {place} hoards rare steel. Bring it down."),
];

pub(super) const ALCHEMIST: &[Template] = &[
    t(Fetch, "Moonpetal Herbs", "Moonpetal", "Pick {n} moonpetal herbs where they glow."),
    t(Hunt(WOLF), "Wolf Fangs", "", "My tonic needs fresh fangs. Hunt {n} wolves."),
    t(Scout, "Strange Vapours", "", "Odd vapours drift from {place}. Go take a look."),
];

pub(super) const MERCHANT: &[Template] = &[
    t(Courier, "Trade Ledger", "", "Carry this ledger to {place}. Time is money!"),
    t(Fetch, "Lost Cargo", "Spice crate", "Bandits scattered my crates. Recover {n} of them."),
];

pub(super) const GUARD: &[Template] = &[
    t(Patrol, "Night Watch", "", "Walk the {n} watch posts around the village and report back."),
    t(Hunt(WOLF), "Wolf Pack", "", "A pack is prowling close. Thin it out by {n}."),
    t(Slay, "Threat Beyond the Walls", "", "The {boss} of {place} grows bolder. End it."),
];

pub(super) const INNKEEPER: &[Template] = &[
    t(Hunt(PIG), "Feast Supplies", "", "The cellar is empty. Bring meat from {n} pigs."),
    t(Courier, "Invitations", "", "Take this invitation to the inn at {place}."),
];

pub(super) const HUNTER: &[Template] = &[
    t(Hunt(DEER), "Deer Season", "", "Prove your aim: hunt {n} deer."),
    t(Hunt(None), "Thinning the Wilds", "", "Too many beasts this season. Hunt {n} of anything."),
    t(Hunt(WOLF), "The Alpha's Pack", "", "That pack took my best hound. Slay {n} wolves."),
];

pub(super) const COOK: &[Template] = &[
    t(Hunt(PIG), "Roast for the Table", "", "Bring me {n} pigs for tonight's roast."),
    t(Fetch, "Wild Mushrooms", "Mushroom", "Forage {n} mushrooms at the marked glade."),
];

pub(super) const MINER: &[Template] = &[
    t(Fetch, "Lost Pickaxes", "Pickaxe", "My crew dropped their picks running away. Recover {n}."),
    t(Scout, "Deep Rumbling", "", "We heard rumbling near {place}. Scout it for us."),
];

pub(super) const FISHERMAN: &[Template] = &[
    t(Fetch, "Torn Nets", "Fishing net", "A storm tore my nets loose. Find {n} of them."),
    t(Courier, "Fresh Catch", "", "Rush this catch to {place} before it spoils."),
];

pub(super) const SCHOLAR: &[Template] = &[
    t(Scout, "Ruins Survey", "", "Survey {place} and tell me what you saw."),
    t(Fetch, "Scattered Pages", "Old page", "The wind took my notes! Collect {n} pages."),
    t(Courier, "Letter to a Colleague", "", "Bring this letter to my colleague in {place}."),
];

pub(super) const PRIEST: &[Template] = &[
    t(Patrol, "Blessing the Shrines", "", "Bless the {n} shrines around the village."),
    t(Fetch, "Pilgrim Relics", "Relic", "Relics were lost on the pilgrim path. Recover {n}."),
    t(Slay, "Cleansing", "", "Darkness festers in {place}. Purge the {boss}."),
];

pub(super) const BARD: &[Template] = &[
    t(Courier, "Travelling Song", "", "Carry my new ballad to {place}."),
    t(Scout, "Tales of Heroes", "", "Visit {place} so I have something worth singing about."),
];

pub(super) const NOBLE: &[Template] = &[
    t(Slay, "Elder's Decree", "", "The {boss} of {place} threatens us all. Slay it and be rewarded."),
    t(Patrol, "Inspect the Borders", "", "Inspect the {n} border stones for me."),
    t(Courier, "Sealed Letter", "", "Deliver this sealed letter to the elder of {place}."),
];

pub(super) static GENERIC: Template = t(Hunt(None), "Helping Hand", "", "Wild beasts trouble us. Hunt {n} of them.");

pub(super) fn templates(job: Profession) -> &'static [Template] {
    match job {
        Profession::Farmer => FARMER,
        Profession::Blacksmith => BLACKSMITH,
        Profession::Alchemist => ALCHEMIST,
        Profession::Merchant => MERCHANT,
        Profession::Guard => GUARD,
        Profession::Innkeeper => INNKEEPER,
        Profession::Hunter => HUNTER,
        Profession::Cook => COOK,
        Profession::Miner => MINER,
        Profession::Fisherman => FISHERMAN,
        Profession::Scholar => SCHOLAR,
        Profession::Priest => PRIEST,
        Profession::Bard => BARD,
        Profession::Noble => NOBLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redstoric_core::voice::voice_id;

    const JOBS: [Profession; 14] = [
        Profession::Farmer,
        Profession::Blacksmith,
        Profession::Alchemist,
        Profession::Merchant,
        Profession::Guard,
        Profession::Innkeeper,
        Profession::Hunter,
        Profession::Cook,
        Profession::Miner,
        Profession::Fisherman,
        Profession::Scholar,
        Profession::Priest,
        Profession::Bard,
        Profession::Noble,
    ];

    fn all_templates() -> Vec<(Profession, &'static Template)> {
        let mut v = Vec::new();
        for job in JOBS {
            for t in templates(job).iter() {
                v.push((job, t));
            }
        }
        v.push((Profession::Farmer, &GENERIC));
        v
    }

    /// Các `{...}` trong lời thoại: chỉ được dùng đúng những khóa mà code thay thế.
    fn placeholders(text: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let mut rest = text;
        while let Some(start) = rest.find('{') {
            let Some(len) = rest[start..].find('}') else {
                out.push(&rest[start..]);
                break;
            };
            out.push(&rest[start..start + len + 1]);
            rest = &rest[start + len + 1..];
        }
        out
    }

    #[test]
    fn every_job_has_several_distinct_quests() {
        for job in JOBS {
            let list = templates(job);
            assert!(list.len() >= 2, "{job:?} có quá ít nhiệm vụ");
            let mut titles = HashSet::new();
            for t in list.iter() {
                assert!(titles.insert(t.title), "{job:?}: trùng tiêu đề {:?}", t.title);
            }
        }
    }

    #[test]
    fn templates_are_well_formed() {
        for (job, t) in all_templates() {
            let who = format!("{job:?} / {}", t.title);
            assert!(!t.title.trim().is_empty() && !t.intro.trim().is_empty(), "{who}: thiếu chữ");
            assert!(t.title.is_ascii() && t.intro.is_ascii() && t.item.is_ascii(), "{who}: có ký tự ngoài ASCII");
            assert!(t.intro.len() <= 140, "{who}: lời thoại quá dài ({} ký tự)", t.intro.len());
            for p in placeholders(t.intro) {
                assert!(["{n}", "{place}", "{boss}", "{item}"].contains(&p), "{who}: khóa lạ {p}");
            }
            match t.kind {
                Kind::Fetch => {
                    assert!(!t.item.is_empty(), "{who}: Fetch phải có tên vật phẩm");
                    assert!(t.intro.contains("{n}"), "{who}: Fetch phải có {{n}}");
                }
                Kind::Hunt(_) | Kind::Patrol => {
                    assert!(t.intro.contains("{n}"), "{who}: phải có {{n}}");
                }
                Kind::Courier | Kind::Scout => {
                    assert!(t.intro.contains("{place}"), "{who}: phải có {{place}}");
                }
                Kind::Slay => {
                    assert!(t.intro.contains("{boss}") && t.intro.contains("{place}"), "{who}: Slay cần {{boss}} và {{place}}");
                }
            }
        }
    }

    #[test]
    fn voice_ids_do_not_collide() {
        let mut seen: HashMap<String, &str> = HashMap::new();
        for (_, t) in all_templates() {
            if let Some(prev) = seen.insert(voice_id(t.intro), t.intro) {
                assert_eq!(prev, t.intro, "hai câu khác nhau trùng id giọng nói");
            }
        }
    }

    #[test]
    fn quest_intros_are_in_voice_csv() {
        // `assets/` bị .gitignore: không có file thì bỏ qua.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/audio/voice/voice_lines.csv");
        let Ok(csv) = std::fs::read_to_string(path) else { return };
        for (job, t) in all_templates() {
            let id = voice_id(t.intro);
            let prefix = format!("{id},");
            assert!(
                csv.lines().any(|l| l.starts_with(&prefix)),
                "câu nhiệm vụ {:?} của {job:?} (id {id}) chưa có trong voice_lines.csv; chạy: python tools/export_voice_lines.py",
                t.intro
            );
        }
    }
}
