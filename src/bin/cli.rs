//! A stdin-driven console harness for the MVP loop: one human vs one
//! dumb bot, no networking, no wasm. Run with `cargo run --bin cli`
//! from the project root (it loads mods/base relative to the cwd).

use std::io::{self, Write};
use std::path::Path;

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

/// Prints a numbered list of `(hand_index, description)` options and
/// returns the chosen entry's hand_index. If `allow_none`, "0"/blank
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

/// Same idea, but for comma-separated multi-select (combo cards).
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
    let defended = match &res.defended_with {
        Some(name) => format!(" Defended with {name} (DEF {}).", res.def_total),
        None => " No defense.".to_string(),
    };
    println!(
        "{attacker} attack: ATK {} ({color}).{defended} {} damage dealt.",
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
    let registry = match CardRegistry::load_mod(Path::new("mods/base"), "en") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to load base mod: {e}");
            eprintln!("(run this from the project root: `cargo run --bin cli`)");
            return;
        }
    };
    let mut game = GameState::new(registry);

    println!("=== Godwheel: MVP ===");
    println!("You vs a bot. Reduce their HP to 0 to win.");

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
            Some(hand_index) => {
                let card_id = game.human.hand[hand_index].clone();
                let is_attack = game.registry.get(&card_id).map_or(false, |d| d.is_attack());
                if is_attack {
                    let combos = game.combo_options(Side::Human);
                    let combo_indices = if combos.is_empty() {
                        Vec::new()
                    } else {
                        println!("Attach combo cards? (comma-separated numbers, or blank for none)");
                        choose_multi(&combos, "> ")
                    };
                    let mut indices = vec![hand_index];
                    indices.extend(combo_indices);
                    match game.human_attack(&indices) {
                        Ok(res) => print_attack_result(&res),
                        Err(e) => println!("Can't do that: {e}"),
                    }
                } else {
                    match game.human_effect(hand_index) {
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
        if let Some(heal_idx) = game.bot_plan_heal() {
            match game.bot_use_effect(heal_idx) {
                Ok(msg) => println!("\n{msg}"),
                Err(e) => println!("\nBot fumbled: {e}"),
            }
        } else if let Some(atk_idx) = game.bot_plan_attack() {
            let card_id = game.bot.hand[atk_idx].clone();
            println!("\nBot attacks with {}!", game.registry.name_of(&card_id));
            let defense = game.defensive_options(Side::Human);
            let defense_index = if defense.is_empty() {
                None
            } else {
                println!("Defend with?");
                choose_from(&defense, "> ", true)
            };
            match game.resolve_bot_attack(atk_idx, defense_index) {
                Ok(res) => print_attack_result(&res),
                Err(e) => println!("Bot's attack fizzled: {e}"),
            }
        } else {
            println!("\nBot passes.");
        }
    }
}
