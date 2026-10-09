//! Only the rows near the scroll position are drawn.

use super::*;
use iced::widget::text;
use iced_test::simulator;

fn list(heights: &Heights, offset: f32) -> Element<'static, (u64, f32)> {
    let rows = (0..100).map(|i| (row_key(&i), i));
    let window = Window {
        offset,
        screen: 100.0,
    };
    lazy_list(
        rows,
        heights,
        50.0,
        window,
        |_, i| text(format!("row {i}")).into(),
        |k, h| (k, h),
    )
}

#[test]
fn rows_far_from_the_screen_are_not_drawn() {
    let heights = Heights::default();
    // At the top: three screens of 50-pixel rows below are drawn, nothing more.
    let mut ui = simulator(list(&heights, 0.0));
    assert!(ui.find("row 0").is_ok());
    assert!(ui.find("row 6").is_ok());
    assert!(ui.find("row 7").is_err());

    // Scrolled to row 40: two screens above it are drawn, rows before that aren't.
    let mut ui = simulator(list(&heights, 2000.0));
    assert!(ui.find("row 35").is_ok());
    assert!(ui.find("row 30").is_err());
    assert!(ui.find("row 45").is_ok());
    assert!(ui.find("row 47").is_err());
}

#[test]
fn measured_heights_replace_the_guess() {
    let mut heights = Heights::default();
    for i in 0..10 {
        heights.set(row_key(&i), 10.0);
    }
    // The first ten rows take 100 pixels in all, so more of them fit.
    let mut ui = simulator(list(&heights, 0.0));
    assert!(ui.find("row 9").is_ok());
    assert!(ui.find("row 13").is_ok());
    heights.retain(|k| k == row_key(&0));
    assert_eq!(heights.len(), 1);
}
