//! TUI renderer for user journey diagrams.
//!
//! Renders journey tasks grouped by section, with score bars and actor info.
//! Each task shows its score as a filled bar (1-5 scale) and involved actors.

use crate::diagrams::journey::JourneyDb;
use crate::error::Result;

const SCORE_MAX: i32 = 5;
const BAR_CHAR: char = '█';
const EMPTY_CHAR: char = '░';

/// Render a user journey diagram as character art.
pub fn render_journey_tui(db: &JourneyDb) -> Result<String> {
    let tasks = db.get_tasks();
    if tasks.is_empty() {
        if !db.title.is_empty() {
            return Ok(format!("{}\n\n(empty journey)\n", db.title));
        }
        return Ok("(empty journey)\n".to_string());
    }

    let mut lines: Vec<String> = Vec::new();

    // Title
    if !db.title.is_empty() {
        lines.push(db.title.clone());
        lines.push("─".repeat(db.title.chars().count().max(40)));
    }

    // Find max task name length for alignment
    let max_task_len = tasks
        .iter()
        .map(|t| t.task.chars().count())
        .max()
        .unwrap_or(0);

    let mut current_section = String::new();

    for task in tasks {
        // Section header
        if task.section != current_section {
            current_section = task.section.clone();
            if !lines.is_empty() {
                lines.push(String::new());
            }
            lines.push(format!("  § {}", current_section));
            lines.push(format!(
                "  {}",
                "─".repeat(current_section.chars().count() + 2)
            ));
        }

        // Score bar
        let filled = task.score.clamp(0, SCORE_MAX) as usize;
        let empty = (SCORE_MAX as usize).saturating_sub(filled);
        let bar: String = std::iter::repeat_n(BAR_CHAR, filled)
            .chain(std::iter::repeat_n(EMPTY_CHAR, empty))
            .collect();

        // Actors
        let actors = if task.people.is_empty() {
            String::new()
        } else {
            format!(" ({})", task.people.join(", "))
        };

        lines.push(format!(
            "    {:width$} │{} {}/{}{}",
            task.task,
            bar,
            task.score,
            SCORE_MAX,
            actors,
            width = max_task_len,
        ));
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_journey(title: &str, sections: &[(&str, &[(&str, i32, &[&str])])]) -> JourneyDb {
        let mut db = JourneyDb::new();
        db.title = title.to_string();
        for (section, tasks) in sections {
            db.add_section(section);
            for (task, score, actors) in *tasks {
                let actors_str = actors.join(", ");
                let data = format!(":{}: {}", score, actors_str);
                db.add_task(task, &data);
            }
        }
        db
    }

    #[test]
    fn empty_journey() {
        let db = JourneyDb::new();
        let output = render_journey_tui(&db).unwrap();
        assert!(output.contains("empty journey"));
    }

    #[test]
    fn title_appears() {
        let db = make_journey(
            "My working day",
            &[("Go to work", &[("Make tea", 5, &["Me"])])],
        );
        let output = render_journey_tui(&db).unwrap();
        assert!(output.contains("My working day"));
    }

    #[test]
    fn sections_appear() {
        let db = make_journey(
            "Day",
            &[
                ("Morning", &[("Wake up", 3, &["Me"])]),
                ("Evening", &[("Sleep", 5, &["Me"])]),
            ],
        );
        let output = render_journey_tui(&db).unwrap();
        assert!(output.contains("Morning"), "Output:\n{}", output);
        assert!(output.contains("Evening"), "Output:\n{}", output);
    }

    #[test]
    fn tasks_and_scores_appear() {
        let db = make_journey(
            "Day",
            &[("Work", &[("Code", 4, &["Dev"]), ("Review", 2, &["Dev"])])],
        );
        let output = render_journey_tui(&db).unwrap();
        assert!(output.contains("Code"), "Output:\n{}", output);
        assert!(output.contains("Review"), "Output:\n{}", output);
        assert!(output.contains("4/5"), "Output:\n{}", output);
        assert!(output.contains("2/5"), "Output:\n{}", output);
    }

    #[test]
    fn actors_appear() {
        let db = make_journey("Day", &[("Work", &[("Pair", 5, &["Alice", "Bob"])])]);
        let output = render_journey_tui(&db).unwrap();
        assert!(output.contains("Alice"), "Output:\n{}", output);
        assert!(output.contains("Bob"), "Output:\n{}", output);
    }

    #[test]
    fn score_bars_proportional() {
        let db = make_journey(
            "Day",
            &[("Work", &[("Great", 5, &["Me"]), ("Bad", 1, &["Me"])])],
        );
        let output = render_journey_tui(&db).unwrap();
        let great_line = output.lines().find(|l| l.contains("Great")).unwrap();
        let bad_line = output.lines().find(|l| l.contains("Bad")).unwrap();
        let count_filled = |line: &str| line.chars().filter(|&c| c == BAR_CHAR).count();
        assert!(count_filled(great_line) > count_filled(bad_line));
    }

    #[test]
    fn gallery_journey_renders() {
        let input = std::fs::read_to_string("docs/sources/journey.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Journey(db) => db,
            _ => panic!("Expected journey diagram"),
        };
        let output = render_journey_tui(&db).unwrap();
        assert!(output.contains("Make tea"), "Output:\n{}", output);
        assert!(output.contains("Go upstairs"), "Output:\n{}", output);
    }
}
