#[derive(Clone, Copy)]
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

const MORTAL_ELEMENTS: [Element; 6] = [
    Element::Fire,
    Element::Water,
    Element::Earth,
    Element::Wind,
    Element::Metal,
    Element::Wood,
];

const DIVINE_ELEMENTS: [Element; 6] = [
    Element::Light,
    Element::Dark,
    Element::Time,
    Element::Space,
    Element::Order,
    Element::Chaos,
];

/// When constructing an attack,
/// combine the elements to determine the final Aspect.
pub fn combine_elements_for_attack(elements: Vec<Option<Element>>) -> Option<Element> {
    elements
        .into_iter()
        .fold(None, combine_two_elements_for_attack)
}
fn combine_two_elements_for_attack(
    element1: Option<Element>,
    element2: Option<Element>,
) -> Option<Element> {
    fn combine(element1: Element, element2: Element) -> Option<Element> {
        todo!("element logic undecided");
    }

    match (element1, element2) {
        (None, None) => None,
        (Some(e1), None) => Some(e1),
        (None, Some(e2)) => Some(e2),
        (Some(e1), Some(e2)) => combine(e1, e2),
    }
}

/// When defending an attack, get the list of aspects that can block it.
pub fn filter_elements_for_defense(incoming_element: Option<Element>) -> Vec<Element> {
    if incoming_element.is_none() {
        // no element - can be blocked by anything
        // todo: maybe include life and death?
        return MORTAL_ELEMENTS
            .iter()
            .chain(DIVINE_ELEMENTS.iter())
            .copied()
            .collect();
    }
    let incoming_element = incoming_element.unwrap();
    // all mortal elements can be blocked by divine elements

    todo!("element logic undecided");
}
