pub mod no_comments;
pub mod plan_style;
pub mod purity;

use crate::Check;

pub const CHECKS: &[&dyn Check] = &[
    &no_comments::NoComments,
    &plan_style::PlanStyle,
    &purity::Purity,
];
