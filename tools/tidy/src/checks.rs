pub mod item_spacing;
pub mod no_comments;
pub mod plan_style;
pub mod purity;
pub mod statement_spacing;

use crate::Check;

pub const CHECKS: &[&dyn Check] = &[
    &item_spacing::ItemSpacing,
    &no_comments::NoComments,
    &plan_style::PlanStyle,
    &purity::Purity,
    &statement_spacing::StatementSpacing,
];
