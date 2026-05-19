use std::sync::Arc;

use destack_artifact::{DirChecked, DirExpanded};
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};

use super::{
    CheckOutput, Condition, Constraint, InferId, InferOrigin, Obligation, StaticInferId,
    StaticInferOrigin,
};

/// Check state for one module graph solve.
#[derive(Debug)]
pub struct CheckState<'a> {
    /// The requested module.
    module: ModuleId,
    /// The requested profile.
    profile: ProfileId,
    /// The expanded DIR input for the requested module.
    expanded: &'a DirExpanded,
    /// Type inference variables.
    infer_origins: Vec<InferOrigin>,
    /// Static inference variables.
    static_infer_origins: Vec<StaticInferOrigin>,
    /// Constraints produced by walking DIR.
    constraints: Vec<Constraint>,
    /// Conditions extracted from source control flow.
    conditions: Vec<Condition>,
    /// Obligations produced by walking DIR.
    obligations: Vec<Obligation>,
    /// Outputs committed after solving.
    outputs: Vec<CheckOutput>,
}

impl<'a> CheckState<'a> {
    /// Create check state for one requested module.
    pub fn new(module: ModuleId, profile: ProfileId, expanded: &'a DirExpanded) -> Self {
        Self {
            module,
            profile,
            expanded,
            infer_origins: Vec::new(),
            static_infer_origins: Vec::new(),
            constraints: Vec::new(),
            conditions: Vec::new(),
            obligations: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Return the requested module.
    pub fn module(&self) -> ModuleId {
        self.module
    }

    /// Return the requested profile.
    pub fn profile(&self) -> ProfileId {
        self.profile
    }

    /// Add one type inference variable.
    pub fn push_infer(&mut self, origin: InferOrigin) -> InferId {
        let id = InferId(self.infer_origins.len() as u32);
        self.infer_origins.push(origin);

        id
    }

    /// Add one static inference variable.
    pub fn push_static_infer(&mut self, origin: StaticInferOrigin) -> StaticInferId {
        let id = StaticInferId(self.static_infer_origins.len() as u32);
        self.static_infer_origins.push(origin);

        id
    }

    /// Add one constraint.
    pub fn push_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Return collected constraints.
    pub fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }

    /// Add one condition.
    pub fn push_condition(&mut self, condition: Condition) {
        self.conditions.push(condition);
    }

    /// Return collected conditions.
    pub fn conditions(&self) -> &[Condition] {
        &self.conditions
    }

    /// Add one obligation.
    pub fn push_obligation(&mut self, obligation: Obligation) {
        self.obligations.push(obligation);
    }

    /// Return collected obligations.
    pub fn obligations(&self) -> &[Obligation] {
        &self.obligations
    }

    /// Add one guarded output.
    pub fn push_output(&mut self, output: CheckOutput) {
        self.outputs.push(output);
    }

    /// Finish check state into checked DIR.
    pub fn finish(self) -> DirChecked {
        DirChecked {
            types: Arc::new(dir::TypeSegment::from_base(&self.expanded.types)),
            resolutions: Arc::new(dir::ResolutionSegment::new(self.module)),
            instances: Arc::new(dir::InstanceSegment::new(self.module)),
            relations: Arc::new(dir::RelationSegment::new(self.module)),
            layouts: Arc::new(dir::LayoutSegment::new(self.module)),
            captures: Arc::new(dir::CaptureSegment::new(self.module)),
        }
    }
}
