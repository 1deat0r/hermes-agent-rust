//! Two-stage skill router (the "skill suggestion" cookbook).
//!
//! Ports the live `typesafe-skill-router` plugin recipe: request 1 ranks
//! the whole roster with one `Choice` plus three gate `Noul`s asking
//! whether a skill is wanted at all; request 2 re-asks the same `Choice`
//! over the top three with each candidate's own description, plus one
//! absolute `fits` `Noul` per candidate. Two requests, two thresholds,
//! at most one skill name back. Chunked over the 255-choice API cap with
//! a `none_of_these` outcome per chunk.

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::guardrail::scrub_text;
use crate::questions::{
    validate_choice_answer, validate_noul_answer, ChoiceQuestion, NoulQuestion, Question,
};
use crate::transport::{post_system_one, JEV_MODEL, MAX_CHOICE_OPTIONS};

/// Candidates carried from the first request into the second.
pub const SHORTLIST: usize = 3;
/// Description characters each candidate brings to stage 2.
pub const EXCERPT_CHARS: usize = 700;
/// Gate threshold: mean of the three gate nouls.
pub const GATE_THRESHOLD: f64 = 0.30;
/// Winner's own fits noul must clear this.
pub const FITS_THRESHOLD: f64 = 0.40;
/// Lead the fits leader needs over the Choice winner to override it.
pub const FITS_MARGIN: f64 = 0.15;
/// Chunk size with headroom under the 255-choice API cap.
pub const CHUNK_CHOICES: usize = 240;
/// Per-chunk no-match outcome (only when the roster is chunked).
pub const NONE_OPTION: &str = "none_of_these";
/// A chunk whose P(none) reaches this nominates no candidates.
pub const NONE_THRESHOLD: f64 = 0.50;

pub const CHOICE_INSTRUCTIONS: &str = "Which of these skills, if any, is the right one to load to help with the user's latest request?";
pub const RERANK_INSTRUCTIONS: &str = "Exactly one of these skills is the right one to load for the user's latest request. Which one? Read what each actually does, not just its name.";

/// Gate questions: is a skill wanted at all?
pub const GATE_QUESTIONS: &[(&str, &str)] = &[
    (
        "acts_on_user_system",
        "Is the assistant being asked to act on the user's files, accounts, devices, or online services, rather than only to explain or advise?",
    ),
    (
        "would_follow_documented_procedure",
        "Would a careful expert answering this consult a specific documented procedure or set of commands, rather than answering from general understanding?",
    ),
    (
        "prose_suffices",
        "Could a knowledgeable generalist fully satisfy this request in prose, with no tools, no documentation, and no access to the user's files or accounts?",
    ),
];
/// A yes here points away from needing a skill.
pub const INVERTED_GATES: &[&str] = &["prose_suffices"];

/// One roster entry.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
}

/// Suggestion: at most one skill name plus the evidence.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Suggestion {
    pub names: Vec<String>,
    pub gate: f64,
    pub shortlist: Vec<String>,
    pub fits: BTreeMap<String, f64>,
    pub winner: Option<String>,
    pub reason: String,
}

impl Suggestion {
    pub fn skill(&self) -> Option<&str> {
        self.names.first().map(String::as_str)
    }
}

/// Split the roster into question-sized chunks (headroom under the cap).
pub fn chunk_roster<'a>(roster: &'a [SkillEntry], size: usize) -> Vec<&'a [SkillEntry]> {
    assert!(size <= MAX_CHOICE_OPTIONS, "chunk size exceeds the API cap");
    if roster.is_empty() {
        return vec![];
    }
    roster.chunks(size).collect()
}

/// Stable sort: probability desc, name asc (bit-stable replays).
pub fn stable_rank(pairs: &mut [(String, f64)]) {
    pairs.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
}

/// Combine stage-2 Choice winner + fits nouls into at most one name.
///
/// The winner is suggested when it is (or ties) the fits argmax and
/// clears `fits_threshold`. A fits leader takes over only by clearing
/// the bar AND leading the winner by `fits_margin`; anything less is
/// two signals disagreeing, and a wrong name costs more than silence.
pub fn resolve_suggestion(
    winner: Option<&str>,
    fits: &BTreeMap<String, f64>,
    fits_threshold: f64,
    fits_margin: f64,
) -> (Vec<String>, String) {
    let Some(winner) = winner else {
        return (vec![], "stage 2 picked nothing: no skill fits".to_string());
    };
    let winner_fits = fits.get(winner).copied().unwrap_or(0.0);
    let mut ranked: Vec<(&String, f64)> = fits.iter().map(|(k, v)| (k, *v)).collect();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(b.0))
    });
    let (best_name, best_fits) = ranked
        .first()
        .map(|(n, v)| (n.as_str(), *v))
        .unwrap_or((winner, 0.0));
    if winner_fits >= best_fits {
        if winner_fits < fits_threshold {
            return (
                vec![],
                format!(
                    "winner {winner} fits {winner_fits:.2} < {fits_threshold:.2}: nothing fits"
                ),
            );
        }
        return (
            vec![winner.to_string()],
            format!("shortlist winner with fits {winner_fits:.2}"),
        );
    }
    if best_fits >= fits_threshold && best_fits - winner_fits >= fits_margin {
        return (
            vec![best_name.to_string()],
            format!(
                "fits override: {best_name} {best_fits:.2} leads winner {winner} {winner_fits:.2}"
            ),
        );
    }
    if best_fits < fits_threshold {
        (
            vec![],
            format!("winner {winner} fits {winner_fits:.2}, best {best_name} {best_fits:.2} < {fits_threshold:.2}: nothing fits"),
        )
    } else {
        (
            vec![],
            format!("choice picked {winner} but {best_name} leads under the {fits_margin:.2} margin, staying silent"),
        )
    }
}

/// Stage-1 questions for one chunk (+ gate nouls on the first chunk).
pub fn rank_questions(
    chunk: &[SkillEntry],
    with_gates: bool,
    chunked: bool,
) -> BTreeMap<String, Question> {
    let mut criteria: BTreeMap<String, Option<String>> = chunk
        .iter()
        .map(|s| {
            (
                s.name.clone(),
                Some(s.description.chars().take(60).collect::<String>()),
            )
        })
        .collect();
    if chunked {
        criteria.insert(
            NONE_OPTION.to_string(),
            Some("None of these skills fit the request.".to_string()),
        );
    }
    let mut questions = BTreeMap::new();
    questions.insert(
        "which".to_string(),
        Question::Choice(ChoiceQuestion {
            instructions: Value::String(CHOICE_INSTRUCTIONS.to_string()),
            criteria,
        }),
    );
    if with_gates {
        for (key, text) in GATE_QUESTIONS {
            questions.insert(
                format!("gate::{key}"),
                Question::Noul(NoulQuestion {
                    instructions: Value::String(text.to_string()),
                    criteria_true: None,
                    criteria_false: None,
                }),
            );
        }
    }
    questions
}

/// Stage-2 questions: Choice over the shortlist + one fits Noul each.
pub fn rerank_questions(shortlist: &[SkillEntry]) -> BTreeMap<String, Question> {
    let mut criteria: BTreeMap<String, Option<String>> = shortlist
        .iter()
        .map(|s| {
            (
                s.name.clone(),
                Some(
                    s.description
                        .chars()
                        .take(EXCERPT_CHARS)
                        .collect::<String>(),
                ),
            )
        })
        .collect();
    criteria.insert(
        NONE_OPTION.to_string(),
        Some("None of these skills fit the request.".to_string()),
    );
    let mut questions = BTreeMap::new();
    questions.insert(
        "which".to_string(),
        Question::Choice(ChoiceQuestion {
            instructions: Value::String(RERANK_INSTRUCTIONS.to_string()),
            criteria,
        }),
    );
    for skill in shortlist {
        questions.insert(
            format!("fits::{}", skill.name),
            Question::Noul(NoulQuestion {
                instructions: Value::String(format!(
                    "Does the skill '{}' do the specific thing the user's request asks for? It is described as: {}",
                    skill.name, skill.description
                )),
                criteria_true: None,
                criteria_false: None,
            }),
        );
    }
    questions
}

/// Two requests, two thresholds, at most one skill name. `None` on any
/// failure (keyless, transport, invalid): the caller runs skill-less.
pub fn suggest(request: &str, roster: &[SkillEntry]) -> Option<Suggestion> {
    suggest_with_model(request, roster, JEV_MODEL)
}

/// Same as [`suggest`] with an explicit model id.
pub fn suggest_with_model(request: &str, roster: &[SkillEntry], model: &str) -> Option<Suggestion> {
    if roster.is_empty() {
        return Some(Suggestion {
            reason: "roster empty".to_string(),
            ..Default::default()
        });
    }
    let state = json!({
        "request": scrub_text(request).chars().take(2000).collect::<String>(),
        "recent_context": "",
    });
    // Stage 1: rank every chunk (+ gates on the first). Per-chunk
    // ranks are captured as they arrive — probabilities from different
    // chunks are not a global ranking, so nomination works per chunk.
    let chunks = chunk_roster(roster, CHUNK_CHOICES);
    let chunked = chunks.len() > 1;
    let mut per_chunk: Vec<Vec<(String, f64)>> = Vec::with_capacity(chunks.len());
    let mut none_pressure: Vec<f64> = Vec::with_capacity(chunks.len());
    let mut gate_values: BTreeMap<String, f64> = BTreeMap::new();
    for (index, chunk) in chunks.iter().enumerate() {
        let questions = rank_questions(chunk, index == 0, chunked);
        let answers = post_system_one(&state, &questions, model).ok()?;
        let ids: Vec<&str> = questions
            .get("which")
            .and_then(|q| match q {
                Question::Choice(c) => Some(c),
                _ => None,
            })
            .map(|c| c.criteria.keys().map(String::as_str).collect::<Vec<_>>())
            .unwrap_or_default();
        let parsed = validate_choice_answer(&ids, answers.get("which")?)?;
        none_pressure.push(
            parsed
                .probabilities
                .get(NONE_OPTION)
                .copied()
                .unwrap_or(0.0),
        );
        let mut ranked: Vec<(String, f64)> = parsed
            .probabilities
            .iter()
            .filter(|(name, _)| name.as_str() != NONE_OPTION)
            .map(|(name, prob)| (name.clone(), *prob))
            .collect();
        stable_rank(&mut ranked);
        per_chunk.push(ranked);
        if index == 0 {
            for (key, _) in GATE_QUESTIONS {
                let answer = answers.get(&format!("gate::{key}"))?;
                gate_values.insert(key.to_string(), validate_noul_answer(answer)?.noul);
            }
        }
    }
    let oriented: Vec<f64> = gate_values
        .iter()
        .map(|(k, v)| {
            if INVERTED_GATES.contains(&k.as_str()) {
                1.0 - v
            } else {
                *v
            }
        })
        .collect();
    let gate = if oriented.is_empty() {
        0.0
    } else {
        oriented.iter().sum::<f64>() / oriented.len() as f64
    };
    if gate < GATE_THRESHOLD {
        return Some(Suggestion {
            gate,
            reason: format!("gate {gate:.2} < {GATE_THRESHOLD:.2}: no skill wanted"),
            ..Default::default()
        });
    }
    // Shortlist: best chunk always nominates; other chunks nominate
    // only while their own P(none) stays under NONE_THRESHOLD.
    let best_chunk = per_chunk
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            let pa = a.first().map(|(_, p)| *p).unwrap_or(-1.0);
            let pb = b.first().map(|(_, p)| *p).unwrap_or(-1.0);
            pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);
    let mut shortlist: Vec<String> = Vec::new();
    for (ci, group) in per_chunk.iter().enumerate() {
        if ci != best_chunk && none_pressure.get(ci).copied().unwrap_or(0.0) >= NONE_THRESHOLD {
            continue;
        }
        for (name, _) in group.iter().take(SHORTLIST) {
            if !shortlist.contains(name) {
                shortlist.push(name.clone());
            }
        }
    }
    if shortlist.is_empty() {
        return Some(Suggestion {
            gate,
            reason: "no chunk nominated a candidate".to_string(),
            ..Default::default()
        });
    }
    // Stage 2: rerank the shortlist with full descriptions + fits nouls.
    let by_name: BTreeMap<&str, &SkillEntry> =
        roster.iter().map(|s| (s.name.as_str(), s)).collect();
    let short_entries: Vec<SkillEntry> = shortlist
        .iter()
        .filter_map(|n| by_name.get(n.as_str()).map(|s| (*s).clone()))
        .collect();
    let questions = rerank_questions(&short_entries);
    let answers = post_system_one(&state, &questions, model).ok()?;
    let ids: Vec<&str> = short_entries
        .iter()
        .map(|s| s.name.as_str())
        .chain(std::iter::once(NONE_OPTION))
        .collect();
    let parsed = validate_choice_answer(&ids, answers.get("which")?)?;
    let winner = if parsed.choice == NONE_OPTION {
        None
    } else {
        Some(parsed.choice.as_str())
    };
    let mut fits = BTreeMap::new();
    for entry in &short_entries {
        let answer = answers.get(&format!("fits::{}", entry.name))?;
        fits.insert(entry.name.clone(), validate_noul_answer(answer)?.noul);
    }
    let (names, reason) = resolve_suggestion(winner, &fits, FITS_THRESHOLD, FITS_MARGIN);
    Some(Suggestion {
        names,
        gate,
        shortlist,
        fits,
        winner: winner.map(str::to_string),
        reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roster(n: usize) -> Vec<SkillEntry> {
        (0..n)
            .map(|i| SkillEntry {
                name: format!("skill_{i:03}"),
                description: format!("does thing {i}"),
            })
            .collect()
    }

    #[test]
    fn chunking_respects_cap_with_headroom() {
        let big = roster(300);
        let chunks = chunk_roster(&big, CHUNK_CHOICES);
        assert_eq!(chunks.len(), 2);
        assert!(chunks.iter().all(|c| c.len() <= MAX_CHOICE_OPTIONS));
    }

    #[test]
    fn empty_roster_suggests_nothing() {
        let suggestion = suggest("do it", &[]).expect("empty is not failure");
        assert!(suggestion.names.is_empty());
        assert_eq!(suggestion.reason, "roster empty");
    }
}
