//! A scripted demo driving the engine through a mini-game, to show the whole
//! stack working together. Run with `cargo run`.

use magicbot_engine::carddb::{sample_pool, CardDb};
use magicbot_engine::ids::PlayerId;
use magicbot_engine::layers::Pt;
use magicbot_engine::state::GameState;

/// Render the battlefield as a readable summary (one line per permanent).
fn render_board(s: &GameState, db: &CardDb) -> String {
    let mut lines: Vec<String> = s
        .battlefield_ids()
        .into_iter()
        .map(|id| {
            let chars = s.characteristics(id, db);
            let pt = match chars.and_then(|c| c.pt) {
                Some(Pt { power, toughness }) => format!("{power}/{toughness}"),
                None => "-".to_string(),
            };
            // def name via the object's def_id
            let name = s
                .get(id)
                .and_then(|o| db.get(magicbot_engine::carddb::DefId(o.def_id())))
                .map(|d| d.name().to_string())
                .unwrap_or_else(|| "?".to_string());
            format!("  {name} [{pt}]")
        })
        .collect();
    lines.sort(); // deterministic rendering despite unspecified id order
    if lines.is_empty() {
        "  (battlefield empty)".to_string()
    } else {
        lines.join("\n")
    }
}

/// Resolve the top of the stack: both players pass to let it resolve.
fn resolve(s: &mut GameState, db: &CardDb) {
    let _ = s.pass_priority(db);
    let _ = s.pass_priority(db);
}

/// Script a short one-sided mini-game and return the transcript.
fn run_demo() -> String {
    let db = sample_pool();
    let mut s = GameState::new(2, 0);
    let mut out = String::new();
    out.push_str("=== magicbot engine demo ===\n");

    // P0 casts Grizzly Bears (2/2).
    s.cast(1, PlayerId(0), None);
    resolve(&mut s, &db);
    out.push_str("\nAfter casting Grizzly Bears:\n");
    out.push_str(&render_board(&s, &db));
    out.push('\n');

    // P0 casts Serra Angel (4/4 flier).
    s.cast(2, PlayerId(0), None);
    resolve(&mut s, &db);
    out.push_str("\nAfter casting Serra Angel:\n");
    out.push_str(&render_board(&s, &db));
    out.push('\n');

    // Find the Bears on the battlefield (def_id 1) to bolt it.
    let bears = s
        .battlefield_ids()
        .into_iter()
        .find(|&id| s.get(id).map(|o| o.def_id()) == Some(1))
        .expect("bears on battlefield");

    // P0 casts Lightning Bolt (3 damage) at the Bears.
    s.cast(7, PlayerId(0), Some(bears));
    resolve(&mut s, &db);
    // Run the priority loop again so the SBA removes the lethally-damaged Bears.
    resolve(&mut s, &db);
    out.push_str("\nAfter Lightning Bolt kills the Bears:\n");
    out.push_str(&render_board(&s, &db));
    out.push('\n');

    out
}

fn main() {
    println!("{}", run_demo());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_runs_and_shows_the_expected_arc() {
        let t = run_demo();
        // Grizzly Bears appears after the first cast.
        assert!(t.contains("Grizzly Bears [2/2]"), "bears entered as 2/2");
        // Serra Angel appears as a 4/4.
        assert!(t.contains("Serra Angel [4/4]"), "serra entered as 4/4");
        // After the bolt + SBA, the Bears are gone but Serra remains.
        let final_section = t.rsplit("kills the Bears:").next().unwrap();
        assert!(final_section.contains("Serra Angel"), "serra survives");
        assert!(!final_section.contains("Grizzly Bears"), "bears died");
    }
}
