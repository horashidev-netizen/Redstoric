pub fn flat_world_positions(width: i32, depth: i32, y: i32) -> Vec<(i32, i32, i32)> {
    (0..width)
        .flat_map(|x| (0..depth).map(move |z| (x, y, z)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::flat_world_positions;

    #[test]
    fn flat_world_has_expected_size() {
        let positions = flat_world_positions(16, 16, 0);
        assert_eq!(positions.len(), 256);
    }

    #[test]
    fn flat_world_all_blocks_share_same_height() {
        let positions = flat_world_positions(4, 3, 2);
        assert!(positions.iter().all(|(_, y, _)| *y == 2));
    }
}
