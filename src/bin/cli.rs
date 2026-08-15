//! A stdin-driven console harness for the MVP loop: one human vs one
//! dumb bot, no networking, no wasm. Run with `cargo run --bin cli`
//! from anywhere -- the base mod is baked into the binary, no cwd
//! dependency.

use std::io::{self, Write};

use godwheel_2::game::{AttackResult, GameState, Side};
use godwheel_2::registry::CardRegistry;
use godwheel_2::wheel::DisplayColor;

fn read_line(prompt: &str) -> String {
    print!("{prompt}");
    io::stdout().flush().ok();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
    buf.trim().to_string()
}

/// Prints a numbered list of `(hand_slot, description)` options and
/// returns the chosen entry's hand_slot. If `allow_none`, "0"/blank
/// returns None (pass / no combo / no defense).
fn choose_from(options: &[(usize, String)], prompt: &str, allow_none: bool) -> Option<usize> {
    for (i, (_, desc)) in options.iter().enumerate() {
        println!("  {}) {}", i + 1, desc);
    }
    if allow_none {
        println!("  0) none");
    }
    loop {
        let input = read_line(prompt);
        if allow_none && (input == "0" || input.is_empty()) {
            return None;
        }
        if let Ok(n) = input.parse::<usize>() {
            if n >= 1 && n <= options.len() {
                return Some(options[n - 1].0);
            }
        }
        println!("  invalid choice, try again.");
    }
}

/// Same idea, but for comma-separated multi-select (combos / defense).
fn choose_multi(options: &[(usize, String)], prompt: &str) -> Vec<usize> {
    if options.is_empty() {
        return Vec::new();
    }
    for (i, (_, desc)) in options.iter().enumerate() {
        println!("  {}) {}", i + 1, desc);
    }
    let input = read_line(prompt);
    input
        .split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n >= 1 && n <= options.len())
        .map(|n| options[n - 1].0)
        .collect()
}

fn choose_target() -> Side {
    println!("  1) Yourself   2) Bot");
    loop {
        match read_line("target> ").as_str() {
            "1" => return Side::Human,
            "2" => return Side::Bot,
            _ => println!("  invalid choice, try again."),
        }
    }
}

fn print_status(game: &GameState) {
    println!(
        "\nYou: HP {} MP {} $ {}   |   Bot: HP {} MP {} $ {}",
        game.human.health,
        game.human.mana,
        game.human.money,
        game.bot.health,
        game.bot.mana,
        game.bot.money,
    );
}

fn print_attack_result(res: &AttackResult) {
    let color = match res.color {
        DisplayColor::Neutral => "black".to_string(),
        DisplayColor::Element(e) => format!("{e:?}").to_lowercase(),
    };
    let attacker = match res.attacker {
        Side::Human => "You",
        Side::Bot => "Bot",
    };
    let target = match res.target {
        Side::Human => "You",
        Side::Bot => "Bot",
    };
    let defended = if res.defended_with.is_empty() {
        " No defense.".to_string()
    } else {
        format!(" Defended with {} (DEF {}).", res.defended_with.join(", "), res.def_total)
    };
    println!(
        "{attacker} -> {target}: ATK {} ({color}).{defended} Net change: {} HP.",
        res.atk_total, res.damage
    );
    if let Some(name) = &res.revived_by {
        println!("  -- that would have been lethal, but {name} triggered and saved them!");
    }
}

fn print_winner(game: &GameState) -> bool {
    match game.winner() {
        Some(Side::Human) => {
            println!("\nYou win!");
            true
        }
        Some(Side::Bot) => {
            println!("\nYou lose...");
            true
        }
        None => false,
    }
}

fn main() {
    let registry = CardRegistry::load_embedded_base();
    let mut game = GameState::new(registry);

    println!("=== Godwheel: MVP ===");
    println!("You vs a bot. Reduce their HP to 0 to win. (Yes, you can attack yourself.)");

    loop {
        if print_winner(&game) {
            break;
        }

        print_status(&game);

        let offense = game.offensive_options(Side::Human);
        println!("\nYour turn -- choose a card:");
        let chosen = choose_from(&offense, "> ", true);

        match chosen {
            None => println!("You pass."),
            Some(slot) => {
                let card_id = game.human.hand[slot].clone().unwrap();
                let is_combat = game.registry.get(&card_id).map_or(false, |d| d.is_combat_card());
                if is_combat {
                    let combos = game.combo_options(Side::Human);
                    let combo_slots = if combos.is_empty() {
                        Vec::new()
                    } else {
                        println!("Attach combo cards? (comma-separated numbers, or blank for none)");
                        choose_multi(&combos, "> ")
                    };
                    println!("Target:");
                    let target = choose_target();
                    let mut indices = vec![slot];
                    indices.extend(combo_slots);
                    match game.human_play(&indices, target) {
                        Ok(res) => print_attack_result(&res),
                        Err(e) => println!("Can't do that: {e}"),
                    }
                } else {
                    match game.human_effect(slot) {
                        Ok(msg) => println!("{msg}"),
                        Err(e) => println!("Can't do that: {e}"),
                    }
                }
            }
        }

        if print_winner(&game) {
            break;
        }

        // ---- bot's turn ----
        match game.bot_plan_turn() {
            None => println!("\nBot passes."),
            Some((slots, target, atk)) => {
                let names: Vec<String> = slots
                    .iter()
                    .filter_map(|&s| game.bot.hand[s].clone())
                    .map(|id| game.registry.name_of(&id).to_string())
                    .collect();
                let target_desc = if target == Side::Human { "you" } else { "itself" };
                println!("\nBot plays {} (ATK {atk}) targeting {target_desc}!", names.join(" + "));

                let defense_slots = if target == Side::Human {
                    let defense = game.defensive_options(Side::Human);
                    if defense.is_empty() {
                        Vec::new()
                    } else {
                        println!("Defend with? (comma-separated numbers, or blank for none)");
                        choose_multi(&defense, "> ")
                    }
                } else {
                    Vec::new()
                };

                match game.bot_resolve_turn(&slots, target, &defense_slots) {
                    Ok(res) => print_attack_result(&res),
                    Err(e) => println!("Bot's move fizzled: {e}"),
                }
            }
        }
    }
}
