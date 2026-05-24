use destack_artifact::GlobalEnvironment;
use destack_dir as dir;

use crate::check::{CheckModuleState, MemberOutcome, MemberResolutionTarget};

impl CheckModuleState {
    /// Commit member outcomes into the checked resolution table.
    pub(super) fn commit_member(&mut self, environment: &GlobalEnvironment) {
        let members = self
            .decisions
            .member
            .values()
            .filter_map(|outcome| match outcome {
                MemberOutcome::Resolved(member) => Some(member),
                MemberOutcome::Rejected(_) => None,
            })
            .cloned()
            .collect::<Vec<_>>();

        // write member outcomes recorded by solve
        for member in members {
            if self
                .output
                .resolutions
                .member_resolution(member.source)
                .is_some()
            {
                continue;
            }
            let Some(receiver) = self.commit_variable_type(environment, member.receiver) else {
                continue;
            };
            let target = match member.target {
                MemberResolutionTarget::Field(key) => dir::MemberTarget::Field(key),
                MemberResolutionTarget::Symbol { symbol, instance } => {
                    let instance = instance.as_ref().and_then(|instance| {
                        self.commit_generic_instance(environment, member.source, instance)
                    });
                    let candidate = dir::MemberCandidate {
                        receiver: Some(receiver),
                        symbol,
                        instance,
                    };

                    dir::MemberTarget::Symbol(candidate)
                }
            };
            let resolution = dir::MemberResolution::new(Some(receiver), target);

            self.output
                .resolutions
                .set_member_resolution(member.source, resolution);
        }
    }
}
