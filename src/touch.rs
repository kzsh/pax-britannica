//! Touchscreen input: the screen split down the middle into two one-button
//! controllers.
//!
//! The original game has no touch support at all; this is a browser affordance,
//! not a port of anything. It stays out of the simulation entirely -- the
//! platform layer folds whatever this returns into
//! [`TheOneButton::keys`](crate::the_one_button::TheOneButton::keys) alongside
//! the keyboard, so the golden trace is unaffected.
//!
//! Left half is player 1 and right half is player 2, which mirrors the join
//! screen: a tap on a half joins that player, and the two thumbs then keep the
//! halves they joined with. A lone human gains a CPU opponent, so their half
//! would leave the other one dead; instead the whole screen becomes theirs.

use crate::the_one_button::PLAYERS;

/// Which players the on-screen halves drive, given the human roster.
///
/// `humans` is the player numbers of the human-controlled factories in the
/// running match, ascending, and empty on the menu.
fn halves(humans: &[usize]) -> (usize, usize) {
    match humans {
        // the menu: either half joins its player
        [] => (1, 2),
        // one human, so the other factory is the CPU's; both halves are theirs
        [only] => (*only, *only),
        [left, right, ..] => (*left, *right),
    }
}

/// The button state each player gets from the pointers currently down.
///
/// `pointers` are horizontal positions as a fraction of the window's width;
/// halves of the *window*, not of the letterboxed play area, because the thumb
/// rests on glass rather than on the game.
pub fn buttons(pointers: &[f64], humans: &[usize]) -> [bool; PLAYERS] {
    let (left, right) = halves(humans);

    let mut keys = [false; PLAYERS];
    for &x in pointers {
        let player = if x < 0.5 { left } else { right };
        keys[player - 1] = true;
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
    fn the_menu_joins_the_player_whose_half_was_tapped() {
        assert_eq!(buttons(&[0.25], &[]), [true, false, false, false]);
        assert_eq!(buttons(&[0.75], &[]), [false, true, false, false]);
    }

    #[test]
    fn the_halves_meet_at_the_middle() {
        // exactly half way belongs to the right, and either side of it splits
        assert_eq!(buttons(&[0.5], &[]), [false, true, false, false]);
        assert_eq!(buttons(&[0.499_999], &[]), [true, false, false, false]);
        assert_eq!(buttons(&[0.0], &[]), [true, false, false, false]);
        assert_eq!(buttons(&[1.0], &[]), [false, true, false, false]);
    }

    #[test]
    fn two_thumbs_press_both_buttons() {
        assert_eq!(buttons(&[0.1, 0.9], &[]), [true, true, false, false]);
        assert_eq!(buttons(&[0.1, 0.9], &[1, 2]), [true, true, false, false]);
    }

    #[test]
    fn several_pointers_on_one_half_are_one_button() {
        assert_eq!(buttons(&[0.1, 0.2, 0.3], &[1, 2]), [true, false, false, false]);
    }

    #[test]
    fn a_lone_human_owns_the_whole_screen() {
        for x in [0.1, 0.9] {
            assert_eq!(
                buttons(&[x], &[1]),
                [true, false, false, false],
                "player 1 alone, pointer at {x}"
            );
            assert_eq!(
                buttons(&[x], &[2]),
                [false, true, false, false],
                "player 2 alone, pointer at {x}"
            );
        }
    }

    #[test]
    fn a_lone_human_is_pressed_once_however_many_thumbs_land() {
        assert_eq!(buttons(&[0.1, 0.6, 0.95], &[2]), [false, true, false, false]);
    }

    #[test]
    fn the_halves_follow_the_roster_rather_than_the_player_numbers() {
        // players 3 and 4 can only have joined from the keyboard, but if they
        // are the two humans left holding the match, the halves are theirs
        assert_eq!(buttons(&[0.1], &[3, 4]), [false, false, true, false]);
        assert_eq!(buttons(&[0.9], &[3, 4]), [false, false, false, true]);
    }

    #[test]
    fn a_crowd_leaves_the_last_players_to_the_keyboard() {
        // four humans and two halves: the extra two are unreachable by thumb,
        // which is the price of a two-thumb scheme
        assert_eq!(buttons(&[0.1, 0.9], &[1, 2, 3, 4]), [true, true, false, false]);
    }
}
