//! Statically linked bundled provider profiles.
//!
//! Each file mirrors one `plugins/model-providers/<name>/__init__.py` module.
//! The registry calls `register_builtin_profiles()` before the user-plugin
//! loader so user registrations retain upstream last-writer-wins precedence.

#[path = "actual.rs"]
mod actual;
#[path = "ai_gateway.rs"]
mod ai_gateway;
#[path = "alibaba.rs"]
mod alibaba;
#[path = "alibaba_coding_plan.rs"]
mod alibaba_coding_plan;
#[path = "anthropic.rs"]
mod anthropic;
#[path = "arcee.rs"]
mod arcee;
#[path = "azure_foundry.rs"]
mod azure_foundry;
#[path = "bedrock.rs"]
mod bedrock;
#[path = "copilot.rs"]
mod copilot;
#[path = "copilot_acp.rs"]
mod copilot_acp;
#[path = "custom.rs"]
mod custom;
#[path = "deepinfra.rs"]
mod deepinfra;
#[path = "deepseek.rs"]
mod deepseek;
#[path = "fireworks.rs"]
mod fireworks;
#[path = "gemini.rs"]
mod gemini;
#[path = "gmi.rs"]
mod gmi;
#[path = "huggingface.rs"]
mod huggingface;
#[path = "kilocode.rs"]
mod kilocode;
#[path = "minimax.rs"]
mod minimax;
#[path = "nous.rs"]
mod nous;
#[path = "novita.rs"]
mod novita;
#[path = "nvidia.rs"]
mod nvidia;
#[path = "ollama_cloud.rs"]
mod ollama_cloud;
#[path = "openai_codex.rs"]
mod openai_codex;
#[path = "stepfun.rs"]
mod stepfun;
#[path = "vertex.rs"]
mod vertex;
#[path = "xai.rs"]
mod xai;
#[path = "xiaomi.rs"]
mod xiaomi;

use std::path::Path;

use crate::base::ProviderProfile;
use crate::registry::{register_provider, ProviderSource};

pub(crate) fn register_builtin_profiles() {
    register_provider(actual::profile());
    register_provider(ai_gateway::profile());
    register_provider(alibaba::profile());
    register_provider(alibaba_coding_plan::profile());
    register_provider(anthropic::profile());
    register_provider(arcee::profile());
    register_provider(azure_foundry::profile());
    register_provider(bedrock::profile());
    register_provider(copilot::profile());
    register_provider(copilot_acp::profile());
    register_provider(custom::profile());
    register_provider(deepinfra::profile());
    register_provider(deepseek::profile());
    register_provider(fireworks::profile());
    register_provider(gemini::profile());
    register_provider(gmi::profile());
    register_provider(huggingface::profile());
    register_provider(kilocode::profile());
    register_provider(minimax::profile());
    register_provider(minimax::china_profile());
    register_provider(minimax::oauth_profile());
    register_provider(novita::profile());
    register_provider(nvidia::profile());
    register_provider(nous::profile());
    register_provider(ollama_cloud::profile());
    register_provider(openai_codex::profile());
    register_provider(stepfun::profile());
    register_provider(vertex::profile());
    register_provider(xai::profile());
    register_provider(xiaomi::profile());
}

pub(crate) fn load_profile(
    path: &Path,
    source: ProviderSource,
) -> Result<Option<ProviderProfile>, String> {
    if source == ProviderSource::Bundled {
        match path.file_name().and_then(|name| name.to_str()) {
            Some("actual") => return Ok(Some(actual::profile())),
            Some("alibaba") => return Ok(Some(alibaba::profile())),
            Some("alibaba-coding-plan") => return Ok(Some(alibaba_coding_plan::profile())),
            Some("ai-gateway") => return Ok(Some(ai_gateway::profile())),
            Some("anthropic") => return Ok(Some(anthropic::profile())),
            Some("arcee") => return Ok(Some(arcee::profile())),
            Some("azure-foundry") => return Ok(Some(azure_foundry::profile())),
            Some("bedrock") => return Ok(Some(bedrock::profile())),
            Some("copilot") => return Ok(Some(copilot::profile())),
            Some("copilot-acp") => return Ok(Some(copilot_acp::profile())),
            Some("custom") => return Ok(Some(custom::profile())),
            Some("deepinfra") => return Ok(Some(deepinfra::profile())),
            Some("deepseek") => return Ok(Some(deepseek::profile())),
            Some("fireworks") => return Ok(Some(fireworks::profile())),
            Some("gemini") => return Ok(Some(gemini::profile())),
            Some("gmi") => return Ok(Some(gmi::profile())),
            Some("huggingface") => return Ok(Some(huggingface::profile())),
            Some("kilocode") => return Ok(Some(kilocode::profile())),
            Some("minimax") => return Ok(Some(minimax::profile())),
            Some("novita") => return Ok(Some(novita::profile())),
            Some("nvidia") => return Ok(Some(nvidia::profile())),
            Some("nous") => return Ok(Some(nous::profile())),
            Some("ollama-cloud") => return Ok(Some(ollama_cloud::profile())),
            Some("openai-codex") => return Ok(Some(openai_codex::profile())),
            Some("stepfun") => return Ok(Some(stepfun::profile())),
            Some("vertex") => return Ok(Some(vertex::profile())),
            Some("xai") => return Ok(Some(xai::profile())),
            Some("xiaomi") => return Ok(Some(xiaomi::profile())),
            _ => {}
        }
    }
    Ok(None)
}
