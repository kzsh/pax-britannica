//! Touchscreen input: the screen quartered across the middle and down the
//! centre into one-button controllers.
//!
//! The original game has no touch support at all; this is a browser affordance,
//! not a port of anything. It stays out of the simulation entirely -- the
//! platform layer folds whatever this returns into
//! [`TheOneButton::keys`](crate::the_one_button::TheOneButton::keys) alongside
//! the keyboard, so the golden trace is unaffected.
//!
//! The quadrants run in reading order -- top left, top right, bottom left,
//! bottom right -- which is where a four-player match puts each factory
//! (`start_positions`' angles 3pi/4, pi/4, 5pi/4, 7pi/4 in a y-up world), so a
//! thumb sits on the corner it plays. Fewer humans means fewer
//! controllers, so the spare quadrants are folded into the ones in play: two
//! humans get halves down the centre, and a lone human -- who always faces a
//! CPU -- gets the whole screen. Three split the top and share the bottom,
//! matching a three-player match's third factory sitting at the bottom centre.

use crate::the_one_button::PLAYERS;

/// A pointer on the glass: fractions of the window's width and height, y down.
pub type Pointer = (f64, f64);

/// Which player each quadrant drives, in reading order, given the human roster.
///
/// `humans` is the player numbers of the human-controlled factories in the
/// running match, ascending, and empty on the menu.
fn quadrants(humans: &[usize]) -> [usize; 4] {
    match humans {
        // the menu: every quadrant joins its player
        [] => [1, 2, 3, 4],
        [only] => [*only; 4],
        [left, right] => [*left, *right, *left, *right],
        [top_left, top_right, bottom] => [*top_left, *top_right, *bottom, *bottom],
        [a, b, c, d, ..] => [*a, *b, *c, *d],
    }
}

/// The button state each player gets from the pointers currently down.
///
/// Quadrants of the *window*, not of the letterboxed play area, because the
/// thumb rests on glass rather than on the game.
pub fn buttons(pointers: &[Pointer], humans: &[usize]) -> [bool; PLAYERS] {
    let quadrants = quadrants(humans);

    let mut keys = [false; PLAYERS];
    for &(x, y) in pointers {
        let quadrant = usize::from(x >= 0.5) + 2 * usize::from(y >= 0.5);
        keys[quadrants[quadrant] - 1] = true;
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_down_presses_nothing() {
        assert_eq!(buttons(&[], &[]), [false; PLAYERS]);
        assert_eq!(buttons(&[], &[1, 2]), [false; PLAYERS]);
    }

    #[test]
    fn the_menu_joins_the_player_whose_quadrant_was_tapped() {
        assert_eq!(buttons(&[(0.25, 0.25)], &[]), [true, false, false, false]);
        assert_eq!(buttons(&[(0.75, 0.25)], &[]), [false, true, false, false]);
        assert_eq!(buttons(&[(0.25, 0.75)], &[]), [false, false, true, false]);
        assert_eq!(buttons(&[(0.75, 0.75)], &[]), [false, false, false, true]);
    }

    #[test]
    fn the_quadrants_meet_at_the_middle() {
        // dead centre belongs to the bottom right, and either side of it splits
        assert_eq!(buttons(&[(0.5, 0.5)], &[]), [false, false, false, true]);
        assert_eq!(buttons(&[(0.499_999, 0.499_999)], &[]), [true, false, false, false]);
        assert_eq!(buttons(&[(0.0, 0.0)], &[]), [true, false, false, false]);
        assert_eq!(buttons(&[(1.0, 1.0)], &[]), [false, false, false, true]);
    }

    #[test]
    fn four_thumbs_press_four_buttons() {
        let corners = [(0.1, 0.1), (0.9, 0.1), (0.1, 0.9), (0.9, 0.9)];
        assert_eq!(buttons(&corners, &[]), [true; PLAYERS]);
        assert_eq!(buttons(&corners, &[1, 2, 3, 4]), [true; PLAYERS]);
    }

    #[test]
    fn several_pointers_in_one_quadrant_are_one_button() {
        assert_eq!(
            buttons(&[(0.1, 0.1), (0.2, 0.2), (0.3, 0.3)], &[1, 2, 3, 4]),
            [true, false, false, false]
        );
    }

    #[test]
    fn two_humans_split_the_screen_down_the_centre() {
        for y in [0.1, 0.9] {
            assert_eq!(
                buttons(&[(0.1, y)], &[1, 2]),
                [true, false, false, false],
                "left at y {y}"
            );
            assert_eq!(
                buttons(&[(0.9, y)], &[1, 2]),
                [false, true, false, false],
                "right at y {y}"
            );
        }
    }

    #[test]
    fn three_humans_split_the_top_and_share_the_bottom() {
        assert_eq!(buttons(&[(0.1, 0.1)], &[1, 2, 3]), [true, false, false, false]);
        assert_eq!(buttons(&[(0.9, 0.1)], &[1, 2, 3]), [false, true, false, false]);
        for x in [0.1, 0.9] {
            assert_eq!(
                buttons(&[(x, 0.9)], &[1, 2, 3]),
                [false, false, true, false],
                "the bottom half is the third player's, at x {x}"
            );
        }
    }

    #[test]
    fn a_lone_human_owns_the_whole_screen() {
        for pointer in [(0.1, 0.1), (0.9, 0.9)] {
            assert_eq!(
                buttons(&[pointer], &[1]),
                [true, false, false, false],
                "player 1 alone, pointer at {pointer:?}"
            );
            assert_eq!(
                buttons(&[pointer], &[2]),
                [false, true, false, false],
                "player 2 alone, pointer at {pointer:?}"
            );
        }
    }

    #[test]
    fn a_lone_human_is_pressed_once_however_many_thumbs_land() {
        assert_eq!(
            buttons(&[(0.1, 0.1), (0.6, 0.4), (0.95, 0.9)], &[2]),
            [false, true, false, false]
        );
    }

    #[test]
    fn the_quadrants_follow_the_roster_rather_than_the_player_numbers() {
        // players 3 and 4 can only have joined from the keyboard, but if they
        // are the two humans left holding the match, the halves are theirs
        assert_eq!(buttons(&[(0.1, 0.5)], &[3, 4]), [false, false, true, false]);
        assert_eq!(buttons(&[(0.9, 0.5)], &[3, 4]), [false, false, false, true]);
        // and with three left, the survivor at the bottom takes both corners
        assert_eq!(buttons(&[(0.9, 0.9)], &[2, 3, 4]), [false, false, false, true]);
    }
}
