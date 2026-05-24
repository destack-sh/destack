use destack_artifact::GlobalEnvironment;
use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Commit collected receiver resolutions.
    pub(super) fn commit_receiver_resolutions(&mut self, environment: &GlobalEnvironment) {
        let receivers = std::mem::take(&mut self.decisions.receiver);

        // write contextual receiver resolutions
        for receiver in receivers.into_values() {
            let ty = self.commit_variable_type(environment, receiver.ty);
            let resolution = dir::ReceiverResolution {
                kind: receiver.kind,
                owner: receiver.owner,
                ty,
            };

            self.output
                .resolutions
                .set_receiver_resolution(receiver.source, resolution);
        }
    }
}
