//! The pure description of the writes an apply needs.

use std::path::{Path, PathBuf};

use crate::Error;
use crate::model::ids::Target;
use crate::render::wt::WtFragment;

#[allow(missing_docs)]
pub struct PlannedWrite {
    pub target: Target,
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

#[allow(missing_docs)]
pub struct Plan {
    pub writes: Vec<PlannedWrite>,
}

impl Plan {
    #[allow(missing_docs)]
    pub fn fragment(path: &Path, fragment: &WtFragment) -> Result<Self, Error> {
        let mut bytes = serde_json::to_string_pretty(fragment)
            .map_err(|source| Error::JsonEncodeFailed { source })?
            .into_bytes();
        bytes.push(b'\n');
        Ok(Self {
            writes: vec![PlannedWrite {
                target: Target::Wt,
                path: path.to_path_buf(),
                bytes,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Plan;
    use crate::model::ids::Target;
    use crate::render::wt::WtFragment;

    #[test]
    fn a_fragment_plan_ends_with_a_newline() {
        let theme = crate::model::theme::Theme::parse(
            std::path::PathBuf::from("themes/nord.toml"),
            crate::themes::BUNDLED[0].1,
            crate::model::ids::Origin::Bundled,
        )
        .unwrap()
        .validate()
        .unwrap();
        let plan =
            Plan::fragment(Path::new("schemes.json"), &WtFragment::for_theme(&theme)).unwrap();
        assert_eq!(plan.writes.len(), 1);
        assert_eq!(plan.writes[0].target, Target::Wt);
        assert!(plan.writes[0].bytes.ends_with(b"\n"));
    }
}
