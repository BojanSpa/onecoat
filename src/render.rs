pub mod herdr;
pub mod omp;
pub mod wt;

use crate::model::ids::Slot;

pub fn onecoat_name(slot: Slot) -> String {
    format!("onecoat-{}", slot.name())
}
