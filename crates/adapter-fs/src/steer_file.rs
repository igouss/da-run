//! The `## Answer` rule, parity with `bin/steer`'s `section`/`answered?`
//! (`bin/steer:28-38`): the section runs from a line reading exactly
//! `## Answer` to the next `## ` heading or EOF; answered = it holds real text.

/// True when the steer file's `## Answer` section holds non-blank text.
pub fn steer_answered(content: &str) -> bool {
    !answer_section(content).trim().is_empty()
}

fn answer_section(content: &str) -> String {
    let mut inside: bool = false;
    let mut collected: Vec<&str> = Vec::new();
    for line in content.lines() {
        if inside && line.starts_with("## ") {
            break;
        }
        if inside {
            collected.push(line);
        }
        if line.trim_end() == "## Answer" {
            inside = true;
        }
    }
    collected.join("\n")
}

#[cfg(test)]
mod tests {
    use super::steer_answered;

    // The exact sample from bin/steer's selftest.
    const SAMPLE: &str = "# STEER-REQUEST — 02-tests\n\n## Question\n\nWhich port?\n\n## Options\n\n- A: 9080\n- B: 9081\n\n## Answer\n\n";

    #[test]
    fn empty_answer_section_is_unanswered() {
        assert!(!steer_answered(SAMPLE));
    }

    #[test]
    fn whitespace_only_answer_is_unanswered() {
        let content: String = format!("{SAMPLE}  \n");
        assert!(!steer_answered(&content));
    }

    #[test]
    fn real_answer_text_is_answered() {
        let content: String = format!("{SAMPLE}use 9080\n");
        assert!(steer_answered(&content));
    }

    #[test]
    fn missing_answer_heading_is_unanswered() {
        assert!(!steer_answered("# no answer heading"));
    }

    #[test]
    fn answer_section_stops_at_the_next_heading() {
        let content: &str = "## Answer\n\n\n## Later\n\nkeep";
        assert!(!steer_answered(content));
    }
}
