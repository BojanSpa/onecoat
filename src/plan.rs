use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::Error;
use crate::jsonc::Edit;
use crate::model::ids::Target;
use crate::model::theme::{Theme, Validated};
use crate::render::wt;
use crate::targets::Paths;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Absent {
    Create,
    Fail,
}

#[derive(Clone, PartialEq, Debug)]
pub struct PlannedWrite {
    pub target: Target,
    pub path: PathBuf,
    pub edits: Vec<Edit>,
    pub absent: Absent,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Plan {
    pub writes: Vec<PlannedWrite>,
}

impl Plan {
    pub fn wt(paths: &Paths, theme: &Theme<Validated>) -> Result<Self, Error> {
        Ok(Self {
            writes: vec![
                PlannedWrite {
                    target: Target::Wt,
                    path: paths.wt_fragment.clone(),
                    edits: wt::fragment_edits(theme)?,
                    absent: Absent::Create,
                },
                PlannedWrite {
                    target: Target::Wt,
                    path: paths.wt_settings.clone(),
                    edits: wt::settings_edits(theme)?,
                    absent: Absent::Fail,
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
