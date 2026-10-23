mod candidate;
mod location;
mod plan;
mod select;
mod use_def;

use candidate::Candidates;
use fsc_mir::Body;
use use_def::LocalAnalysis;

pub(crate) use location::StatementLocation;
pub(crate) use plan::StackificationPlan;

pub(crate) fn analyze(body: &Body) -> StackificationPlan {
    let locals = LocalAnalysis::collect(body);
    let candidates = Candidates::classify(body, &locals);
    select::select(body, &candidates)
}
