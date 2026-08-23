//! Statically linked bundled provider profiles.
//!
//! Each file mirrors one `plugins/model-providers/<name>/__init__.py` module.
//! The registry calls `register_builtin_profiles()` before the user-plugin
//! loader so user registrations retain upstream last-writer-wins precedence.

#[path = "alibaba.rs"]
mod alibaba;
#[path = "alibaba_coding_plan.rs"]
mod alibaba_coding_plan;
#[path = "arcee.rs"]
mod arcee;
#[path = "azure_foundry.rs"]
mod azure_foundry;
#[path = "huggingface.rs"]
mod huggingface;
#[path = "kilocode.rs"]
mod kilocode;
#[path = "openai_codex.rs"]
mod openai_codex;
#[path = "stepfun.rs"]
mod stepfun;
#[path = "xai.rs"]
mod xai;
#[path = "xiaomi.rs"]
mod xiaomi;

use std::path::Path;

use crate::base::ProviderProfile;
use crate::registry::{register_provider, ProviderSource};

pub(crate) fn register_builtin_profiles() {
    register_provider(alibaba::profile());
    register_provider(alibaba_coding_plan::profile());
    register_provider(arcee::profile());
    register_provider(azure_foundry::profile());
    register_provider(huggingface::profile());
    register_provider(kilocode::profile());
    register_provider(openai_codex::profile());
    register_provider(stepfun::profile());
    register_provider(xai::profile());
    register_provider(xiaomi::profile());
}

pub(crate) fn load_profile(
    path: &Path,
    source: ProviderSource,
) -> Result<Option<ProviderProfile>, String> {
    if source == ProviderSource::Bundled {
        match path.file_name().and_then(|name| name.to_str()) {
            Some("alibaba") => return Ok(Some(alibaba::profile())),
            Some("alibaba-coding-plan") => return Ok(Some(alibaba_coding_plan::profile())),
            Some("arcee") => return Ok(Some(arcee::profile())),
            Some("azure-foundry") => return Ok(Some(azure_foundry::profile())),
            Some("huggingface") => return Ok(Some(huggingface::profile())),
            Some("kilocode") => return Ok(Some(kilocode::profile())),
            Some("openai-codex") => return Ok(Some(openai_codex::profile())),
            Some("stepfun") => return Ok(Some(stepfun::profile())),
            Some("xai") => return Ok(Some(xai::profile())),
            Some("xiaomi") => return Ok(Some(xiaomi::profile())),
            _ => {}
        }
    }
    Ok(None)
}
