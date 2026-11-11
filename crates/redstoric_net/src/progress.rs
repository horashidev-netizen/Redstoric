//! Tiến độ nhân vật được lưu trên server: ví, túi đồ, số nhiệm vụ đã xong, uy tín, dungeon đã hạ.
//!
//! Server chỉ giữ một blob JSON (chưa kiểm chứng, xem README). Client gửi khi có thay đổi
//! và áp lại khi đăng nhập (`Welcome.progress`).

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use redstoric_combat_ai::quest::QuestLog;
use redstoric_core::npc::{Inventory, Wallet};
use serde::{Deserialize, Serialize};

use crate::client::{NetClient, NetStatus};
use crate::protocol::{ClientMsg, CH_RELIABLE};
use bevy_quinnet::client::QuinnetClient;

/// Nhiệm vụ đang làm dở không được lưu (làm lại từ đầu khi vào lại), chỉ lưu kết quả đã đạt.
#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct ProgressSnapshot {
    pub wallet: u32,
    pub inventory: Vec<(String, u32)>,
    pub completed: u32,
    pub issued: u32,
    pub slain_sites: Vec<u32>,
    pub reputation: Vec<(u32, u32)>,
}

impl ProgressSnapshot {
    pub fn capture(wallet: u32, inventory: &[(String, u32)], quests: &QuestLog) -> Self {
        let mut slain_sites: Vec<u32> = quests.slain_sites.iter().copied().collect();
        slain_sites.sort_unstable();
        let mut reputation: Vec<(u32, u32)> = quests.reputation.iter().map(|(k, v)| (*k, *v)).collect();
        reputation.sort_unstable();
        Self { wallet, inventory: inventory.to_vec(), completed: quests.completed, issued: quests.issued, slain_sites, reputation }
    }

    pub fn apply(self, wallet: &mut Wallet, inventory: &mut Inventory, quests: &mut QuestLog) {
        wallet.0 = self.wallet;
        inventory.0 = self.inventory;
        quests.completed = self.completed;
        quests.issued = self.issued;
        quests.slain_sites = self.slain_sites.into_iter().collect::<HashSet<_>>();
        quests.reputation = self.reputation.into_iter().collect::<HashMap<_, _>>();
    }
}

/// Tiến độ nhận từ server, chờ áp vào game.
#[derive(Resource)]
pub struct PendingProgress(pub String);

pub fn apply_pending_progress(
    mut commands: Commands,
    pending: Option<Res<PendingProgress>>,
    wallet: Option<ResMut<Wallet>>,
    inventory: Option<ResMut<Inventory>>,
    quests: Option<ResMut<QuestLog>>,
) {
    let Some(pending) = pending else { return };
    let (Some(mut wallet), Some(mut inventory), Some(mut quests)) = (wallet, inventory, quests) else { return };
    if !pending.0.is_empty() {
        match serde_json::from_str::<ProgressSnapshot>(&pending.0) {
            Ok(snap) => {
                info!("Đã khôi phục tiến độ từ server ({} xu, {} nhiệm vụ xong)", snap.wallet, snap.completed);
                snap.apply(&mut wallet, &mut inventory, &mut quests);
            }
            Err(err) => warn!("Tiến độ lưu trên server không đọc được: {err}"),
        }
    }
    commands.remove_resource::<PendingProgress>();
}

/// Mỗi 5 giây gửi tiến độ lên server nếu có thay đổi.
pub fn send_progress(
    time: Res<Time>,
    mut timer: Local<f32>,
    mut last: Local<String>,
    net: Res<NetClient>,
    mut client: ResMut<QuinnetClient>,
    wallet: Option<Res<Wallet>>,
    inventory: Option<Res<Inventory>>,
    quests: Option<Res<QuestLog>>,
    pending: Option<Res<PendingProgress>>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 || net.status != NetStatus::Online || pending.is_some() {
        return;
    }
    *timer = 5.0;
    let (Some(wallet), Some(inventory), Some(quests)) = (wallet, inventory, quests) else { return };
    let Ok(json) = serde_json::to_string(&ProgressSnapshot::capture(wallet.0, &inventory.0, &quests)) else { return };
    if json == *last {
        return;
    }
    if let Some(conn) = client.get_connection_mut() {
        conn.try_send_message_on(CH_RELIABLE, ClientMsg::Progress(json.clone()));
        *last = json;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trips_through_json() {
        let mut quests = QuestLog::default();
        quests.completed = 4;
        quests.issued = 6;
        quests.slain_sites.insert(9);
        quests.reputation.insert(3, 2);
        let snap = ProgressSnapshot::capture(42, &[("Bread".to_string(), 2)], &quests);
        let json = serde_json::to_string(&snap).unwrap();
        let back: ProgressSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back, snap);

        let (mut w, mut inv, mut q2) = (Wallet(0), Inventory::default(), QuestLog::default());
        back.apply(&mut w, &mut inv, &mut q2);
        assert_eq!(w.0, 42);
        assert_eq!(inv.0, vec![("Bread".to_string(), 2)]);
        assert_eq!((q2.completed, q2.issued), (4, 6));
        assert!(q2.slain_sites.contains(&9));
        assert_eq!(q2.reputation.get(&3), Some(&2));
    }

    #[test]
    fn snapshot_json_stays_under_server_limit_for_typical_play() {
        let mut quests = QuestLog::default();
        for i in 0..60 {
            quests.reputation.insert(i, 5);
            quests.slain_sites.insert(i);
        }
        let inv: Vec<(String, u32)> = (0..30).map(|i| (format!("Item number {i}"), 99)).collect();
        let json = serde_json::to_string(&ProgressSnapshot::capture(100000, &inv, &quests)).unwrap();
        assert!(json.len() < redstoric_store::MAX_PROGRESS_BYTES, "{}", json.len());
    }
}
