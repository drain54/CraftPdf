//! Pixel work for editing a page picture in another app. Nothing here knows about documents, so
//! each step can be checked on its own.

/// Quarter turns clockwise that make a picture placed with `matrix` (its unit square to user
/// space) read upright on a page shown turned `rotation` degrees clockwise. Mirrored or skewed
/// placements turn by whichever way the picture's rows run most.
pub(crate) fn upright_turns(matrix: [f64; 6], rotation: u16) -> u8 {
    let [a, b, ..] = matrix;
    // The way the picture's rows run (left to right), on an unturned screen with y down.
    let (mut x, mut y) = (a, -b);
    for _ in 0..(rotation / 90) % 4 {
        // A quarter turn clockwise on a y-down screen.
        (x, y) = (-y, x);
    }
    if !(x.is_finite() && y.is_finite()) || (x == 0.0 && y == 0.0) {
        return 0;
    }
    if x.abs() >= y.abs() {
        if x > 0.0 { 0 } else { 2 }
    } else if y > 0.0 {
        // Rows run down the screen: the picture shows a quarter turn clockwise; undo it.
        3
    } else {
        1
    }
}

/// `rgba` (straight, 4 bytes a pixel, `w` × `h`) turned `turns` quarter turns clockwise, with
/// its new width and height. `None` when the buffer doesn't hold `w` × `h` pixels.
pub(crate) fn turn_rgba(w: u32, h: u32, rgba: &[u8], turns: u8) -> Option<(u32, u32, Vec<u8>)> {
    let (wu, hu) = (w as usize, h as usize);
    if wu.checked_mul(hu)?.checked_mul(4)? != rgba.len() {
        return None;
    }
    match turns % 4 {
        0 => Some((w, h, rgba.to_vec())),
        2 => Some((w, h, rgba.chunks_exact(4).rev().flatten().copied().collect())),
        t => {
            let mut out = vec![0; rgba.len()];
            // The turned picture is `hu` wide and `wu` tall.
            for ny in 0..wu {
                for nx in 0..hu {
                    let (ox, oy) = if t == 1 { (ny, hu - 1 - nx) } else { (wu - 1 - ny, nx) };
                    let (from, to) = ((oy * wu + ox) * 4, (ny * hu + nx) * 4);
                    out[to..to + 4].copy_from_slice(&rgba[from..from + 4]);
                }
            }
            Some((h, w, out))
        }
    }
}

/// The fewest channels that hold `rgba` without loss, so a grey scan doesn't come back three
/// times its size.
#[derive(Debug, PartialEq)]
pub(crate) enum Packed {
    Gray(Vec<u8>),
    Rgb(Vec<u8>),
    Rgba(Vec<u8>),
}

pub(crate) fn pack(rgba: &[u8]) -> Packed {
    let px = || rgba.chunks_exact(4);
    let opaque = px().all(|p| p[3] == 255);
    if !opaque {
        return Packed::Rgba(rgba.to_vec());
    }
    if px().all(|p| p[0] == p[1] && p[1] == p[2]) {
        Packed::Gray(px().map(|p| p[0]).collect())
    } else {
        Packed::Rgb(px().flat_map(|p| [p[0], p[1], p[2]]).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3 × 2 picture whose pixels are numbered 1..=6 in their red byte.
    fn numbered() -> Vec<u8> {
        (1..=6u8).flat_map(|n| [n, 0, 0, 255]).collect()
    }

    fn reds(px: &[u8]) -> Vec<u8> {
        px.chunks_exact(4).map(|p| p[0]).collect()
    }

    #[test]
    fn turns_move_corners_where_a_turned_page_shows_them() {
        // 1 2 3      clockwise:  4 1     anticlockwise:  3 6     half:  6 5 4
        // 4 5 6                  5 2                     2 5            3 2 1
        //                        6 3                     1 4
        let (w, h, cw) = turn_rgba(3, 2, &numbered(), 1).unwrap();
        assert_eq!((w, h, reds(&cw)), (2, 3, vec![4, 1, 5, 2, 6, 3]));
        let (w, h, ccw) = turn_rgba(3, 2, &numbered(), 3).unwrap();
        assert_eq!((w, h, reds(&ccw)), (2, 3, vec![3, 6, 2, 5, 1, 4]));
        let (_, _, half) = turn_rgba(3, 2, &numbered(), 2).unwrap();
        assert_eq!(reds(&half), vec![6, 5, 4, 3, 2, 1]);
    }

    #[test]
    fn a_turn_and_its_reverse_give_the_picture_back() {
        for t in 0..4u8 {
            let (w, h, there) = turn_rgba(3, 2, &numbered(), t).unwrap();
            let (w, h, back) = turn_rgba(w, h, &there, (4 - t) % 4).unwrap();
            assert_eq!((w, h, back), (3, 2, numbered()), "turns {t}");
        }
    }

    #[test]
    fn a_buffer_of_the_wrong_size_is_refused() {
        assert!(turn_rgba(3, 3, &numbered(), 1).is_none());
        assert!(turn_rgba(u32::MAX, u32::MAX, &numbered(), 1).is_none());
    }

    #[test]
    fn upright_turns_undo_page_rotation_and_turned_placements() {
        let upright = [600.0, 0.0, 0.0, 800.0, 0.0, 0.0];
        assert_eq!(upright_turns(upright, 0), 0);
        // A page shown turned clockwise shows its picture turned clockwise: turn it back.
        assert_eq!(upright_turns(upright, 90), 3);
        assert_eq!(upright_turns(upright, 180), 2);
        assert_eq!(upright_turns(upright, 270), 1);
        // A scan placed a quarter turn anticlockwise on an upright page (rows run up the page).
        let placed_ccw = [0.0, 600.0, -800.0, 0.0, 800.0, 0.0];
        assert_eq!(upright_turns(placed_ccw, 0), 1);
        // ...shown on a page turned clockwise reads upright.
        assert_eq!(upright_turns(placed_ccw, 90), 0);
        assert_eq!(upright_turns([f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0], 0), 0);
    }

    #[test]
    fn pack_keeps_only_the_channels_the_picture_uses() {
        assert_eq!(pack(&[10, 10, 10, 255, 200, 200, 200, 255]), Packed::Gray(vec![10, 200]));
        assert_eq!(pack(&[10, 20, 30, 255]), Packed::Rgb(vec![10, 20, 30]));
        assert_eq!(pack(&[10, 10, 10, 128]), Packed::Rgba(vec![10, 10, 10, 128]));
    }
}
