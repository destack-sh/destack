use tspp_dir as dir;

use super::DirModule;

impl DirModule<'_> {
    /// Visit every authored structural type member list.
    pub(crate) fn visit_type_member_lists<E>(
        &self,
        mut visit: impl FnMut(&[dir::LocalNodeId<dir::TypeMember>]) -> Result<(), E>,
    ) -> Result<(), E> {
        let view = self.view();

        // visit interface member lists
        for (_, declaration) in view.iter_nodes::<dir::Declaration>() {
            if let Some(members) = declaration.type_member_ids() {
                visit(members)?;
            }
        }

        // visit every structural object type, including nested and inline forms
        for (_, expression) in view.iter_nodes::<dir::TypeExpression>() {
            if let dir::TypeExpression::Object { members } = expression {
                visit(members)?;
            }
        }

        Ok(())
    }
}
