use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::Error;
use crate::jsonc::{Edit, Key};
use crate::model::ids::Target;
use crate::model::theme::{Theme, Validated};
use crate::render::wt::{self, ProfileScheme};
use crate::targets::Paths;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Absent {
    Create,
    Fail,
}

#[derive(Clone, PartialEq, Debug)]
pub enum PinReport {
    Report { key: Key, field: String },
    Repoint { key: Key, field: String },
}

#[derive(Clone, PartialEq, Debug)]
pub struct PlannedWrite {
    pub target: Target,
    pub path: PathBuf,
    pub edits: Vec<Edit>,
    pub absent: Absent,
    pub pins: Option<PinReport>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Plan {
    pub writes: Vec<PlannedWrite>,
}

impl Plan {
    pub fn wt(
        paths: &Paths,
        theme: &Theme<Validated>,
        profile_scheme: ProfileScheme,
    ) -> Result<Self, Error> {
        let key = Key::parse(wt::PROFILES_KEY);
        let field = wt::COLOR_SCHEME_FIELD.to_owned();

        let pins = Some(match profile_scheme {
            ProfileScheme::Report => PinReport::Report { key, field },
            ProfileScheme::All => PinReport::Repoint { key, field },
        });

        Ok(Self {
            writes: vec![
                PlannedWrite {
                    target: Target::Wt,
                    path: paths.wt_fragment.clone(),
                    edits: wt::fragment_edits(theme)?,
                    absent: Absent::Create,
                    pins: None,
                },
                PlannedWrite {
                    target: Target::Wt,
                    path: paths.wt_settings.clone(),
                    edits: wt::settings_edits(theme, profile_scheme)?,
                    absent: Absent::Fail,
                    pins,
                },
            ],
        })
    }

    pub fn limited_to(&self, targets: &[Target]) -> Self {
        Self {
            writes: self
                .writes
                .iter()
                .filter(|write| targets.contains(&write.target))
                .cloned()
                .collect(),
        }
    }

    pub fn targets(&self) -> BTreeSet<Target> {
        self.writes.iter().map(|write| write.target).collect()
    }
}
