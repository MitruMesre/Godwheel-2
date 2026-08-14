use serde::Deserialize;

/// Elements are purely cosmetic for the MVP: they decide what color an
/// attack's number is drawn in, and nothing else. No blocking, no
/// resistances, no combining logic. (The full Elemental Wheel from the
/// GDD is deferred until the MVP loop is solid -- this enum stays as the
/// closed, exhaustive primitive set so mods can tag cards with it now
/// without the display logic needing to change later.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    // Mortal
    Fire,
    Water,
    Earth,
    Wind,
    Metal,
    Wood,
    // Divine
    Light,
    Dark,
    Time,
    Space,
    Order,
    Chaos,
    // Other
    Life,
    Death,
}

/// What color should an attack's number be drawn in, given the union of
/// elements contributed by the base card and any combo cards attached to
/// it?
///
/// - No elements at all, or two-or-more *different* elements -> neutral
///   (black).
/// - Exactly one distinct element across every contributing card -> that
///   element's color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayColor {
    Neutral,
    Element(Element),
}

pub fn attack_display_color<'a>(elements: impl IntoIterator<Item = &'a Element>) -> DisplayColor {
    let mut iter = elements.into_iter();
    let Some(&first) = iter.next() else {
        return DisplayColor::Neutral;
    };
    if iter.all(|&e| e == first) {
        DisplayColor::Element(first)
    } else {
        DisplayColor::Neutral
    }
}
