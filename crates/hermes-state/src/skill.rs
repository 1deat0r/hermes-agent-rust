//! agent/skill_commands.py subset — skill-scaffolding recognition used by the
//! state preview shaping. Inlined for Phase 1 (agent crate lands in Phase 2);
//! see the PARITY note in common.rs.
// PARITY: agent/skill_commands.py (constants + describe_skill_invocation +
//         extract_user_instruction_from_skill_message + helpers) @ b9aa928

pub const _SKILL_INVOCATION_PREFIX: &str = "[IMPORTANT: The user has invoked the ";
pub const _SINGLE_SKILL_MARKER: &str = "The full skill content is loaded below.]";
pub const _SINGLE_SKILL_INSTRUCTION: &str =
    "The user has provided the following instruction alongside the skill invocation: ";
pub const _RUNTIME_NOTE: &str = "\n\n[Runtime note:";
pub const _BUNDLE_MARKER: &str = " skill bundle,";
pub const _BUNDLE_USER_INSTRUCTION: &str = "\nUser instruction: ";
pub const _BUNDLE_FIRST_SKILL_BLOCK: &str = "\n\n[Loaded as part of the ";

pub const SKILL_SCAFFOLD_SQL_LIKE: &str = "[IMPORTANT: The user has invoked the %";
pub const SKILL_EXCERPT_JOINT: &str = "\u{1e}"; // ASCII record separator (0x1E)

/// The skill name sits in the first quoted span of the activation note, for
/// both the single-skill and the bundle header ("work" / "/clean /work").
/// Upstream compiles `re.escape(prefix) + r'"([^"]*)"'` and matches at the
/// string start; the prefix contains no quotes, so a manual scan is exact.
// PARITY: agent/skill_commands.py _SKILL_NAME_RE.match @ b9aa928
fn _skill_name_from_content(content: &str) -> Option<String> {
    let rest = content.strip_prefix(_SKILL_INVOCATION_PREFIX)?;
    let inner = rest.strip_prefix('"')?;
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

/// Recover the user's instruction from a slash-skill-expanded turn.
// PARITY: agent/skill_commands.py extract_user_instruction_from_skill_message @ b9aa928
pub fn extract_user_instruction_from_skill_message(content: &str) -> Option<String> {
    if !content.starts_with(_SKILL_INVOCATION_PREFIX) {
        return Some(content.to_string());
    }
    if content.contains(_BUNDLE_MARKER) {
        return _extract_bundle_user_instruction(content);
    }
    if content.contains(_SINGLE_SKILL_MARKER) {
        return _extract_single_skill_user_instruction(content);
    }
    None
}

/// Single-skill format appends the user instruction after the skill body, so
/// the last occurrence is the user-provided one; the body may quote this text.
// PARITY: agent/skill_commands.py _extract_single_skill_user_instruction @ b9aa928
fn _extract_single_skill_user_instruction(message: &str) -> Option<String> {
    let marker_idx = message.rfind(_SINGLE_SKILL_INSTRUCTION)?;
    let mut instruction = &message[marker_idx + _SINGLE_SKILL_INSTRUCTION.len()..];
    if let Some(runtime_idx) = instruction.find(_RUNTIME_NOTE) {
        instruction = &instruction[..runtime_idx];
    }
    let instruction = instruction.trim();
    if instruction.is_empty() {
        None
    } else {
        Some(instruction.to_string())
    }
}

/// Bundle format puts the user instruction before the loaded skills, so the
/// first occurrence is the user-provided one.
// PARITY: agent/skill_commands.py _extract_bundle_user_instruction @ b9aa928
fn _extract_bundle_user_instruction(message: &str) -> Option<String> {
    let marker_idx = message.find(_BUNDLE_USER_INSTRUCTION)?;
    let mut instruction = &message[marker_idx + _BUNDLE_USER_INSTRUCTION.len()..];
    if let Some(first_skill_idx) = instruction.find(_BUNDLE_FIRST_SKILL_BLOCK) {
        instruction = &instruction[..first_skill_idx];
    }
    let instruction = instruction.trim();
    if instruction.is_empty() {
        None
    } else {
        Some(instruction.to_string())
    }
}

/// Render a slash-skill-expanded turn the way the user typed it.
// PARITY: agent/skill_commands.py describe_skill_invocation @ b9aa928
pub fn describe_skill_invocation(content: &str, separator: &str) -> Option<String> {
    if content.is_empty() || !content.starts_with(_SKILL_INVOCATION_PREFIX) {
        return None;
    }
    let name = _skill_name_from_content(content).unwrap_or_default();
    let name = name.trim().to_string();

    let label = if name.starts_with('/') {
        name.clone()
    } else {
        format!("/{}", name)
    };

    let instruction = extract_user_instruction_from_skill_message(content);
    if let Some(inst) = instruction {
        let mut inst = inst;
        inst = inst
            .split(SKILL_EXCERPT_JOINT)
            .next()
            .unwrap_or("")
            .to_string();
        inst = inst.split_whitespace().collect::<Vec<_>>().join(" ");
        if !inst.is_empty() {
            return if name.is_empty() {
                Some(inst)
            } else {
                Some(format!("{label}{separator}{inst}"))
            };
        }
    }
    if name.is_empty() {
        None
    } else {
        Some(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_are_byte_identical_to_upstream() {
        assert_eq!(SKILL_SCAFFOLD_SQL_LIKE, "[IMPORTANT: The user has invoked the %");
        assert_eq!(SKILL_EXCERPT_JOINT, "\u{1e}");
        assert_eq!(_SKILL_INVOCATION_PREFIX, "[IMPORTANT: The user has invoked the ");
        assert_eq!(_SINGLE_SKILL_MARKER, "The full skill content is loaded below.]");
        assert_eq!(_SINGLE_SKILL_INSTRUCTION, "The user has provided the following instruction alongside the skill invocation: ");
        assert_eq!(_RUNTIME_NOTE, "\n\n[Runtime note:");
        assert_eq!(_BUNDLE_MARKER, " skill bundle,");
        assert_eq!(_BUNDLE_USER_INSTRUCTION, "\nUser instruction: ");
        assert_eq!(_BUNDLE_FIRST_SKILL_BLOCK, "\n\n[Loaded as part of the ");
    }

    #[test]
    fn describe_plain_message_returns_none() {
        assert_eq!(describe_skill_invocation("hello world", " — "), None);
        assert_eq!(describe_skill_invocation("", " — "), None);
    }

    #[test]
    fn describe_single_skill() {
        let msg = "[IMPORTANT: The user has invoked the \"work\"\nThe full skill content is loaded below.]\n\nThe user has provided the following instruction alongside the skill invocation: fix the title leak";
        assert_eq!(
            describe_skill_invocation(msg, " — ").as_deref(),
            Some("/work — fix the title leak")
        );
    }

    #[test]
    fn describe_bare_invocation() {
        let msg = "[IMPORTANT: The user has invoked the \"remember\"\nThe full skill content is loaded below.]";
        assert_eq!(describe_skill_invocation(msg, " — ").as_deref(), Some("/remember"));
    }

    #[test]
    fn describe_bundle() {
        let msg = "[IMPORTANT: The user has invoked the \"clean /work\" skill bundle, each skill's full content is loaded below.\nUser instruction: clean up the workspace\n\n[Loaded as part of the ...\n";
        assert_eq!(
            describe_skill_invocation(msg, " — ").as_deref(),
            Some("/clean /work — clean up the workspace")
        );
    }

    #[test]
    fn extract_instruction_returns_original_for_plain() {
        assert_eq!(
            extract_user_instruction_from_skill_message("plain text").as_deref(),
            Some("plain text")
        );
    }
}
