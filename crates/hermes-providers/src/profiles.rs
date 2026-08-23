//! Statically linked bundled provider profiles.
//!
//! Each file mirrors one `plugins/model-providers/<name>/__init__.py` module.
//! The registry calls `register_builtin_profiles()` before the user-plugin
//! loader so user registrations retain upstream last-writer-wins precedence.

#[path = "alibaba.rs"]
mod alibaba;
#[path = "arcee.rs"]
mod arcee;
#[path = "kilocode.rs"]
mod kilocode;
#[path = "stepfun.rs"]
mod stepfun;

use std::path::Path;

use crate::base::ProviderProfile;
use crate::registry::{register_provider, ProviderSource};

pub(crate) fn register_builtin_profiles() {
    register_provider(alibaba::profile());
    register_provider(arcee::profile());
    register_provider(kilocode::profile());
    register_provider(stepfun::profile());
}

pub(crate) fn load_profile(
    path: &Path,
    source: ProviderSource,
) -> Result<Option<ProviderProfile>, String> {
    if source == ProviderSource::Bundled {
        match path.file_name().and_then(|name| name.to_str()) {
            Some("alibaba") => return Ok(Some(alibaba::profile())),
            Some("arcee") => return Ok(Some(arcee::profile())),
            Some("kilocode") => return Ok(Some(kilocode::profile())),
            Some("stepfun") => return Ok(Some(stepfun::profile())),
            _ => {}
        }
    }
    Ok(None)
}
