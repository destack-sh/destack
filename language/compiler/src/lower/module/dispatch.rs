use std::mem;

use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{GenericScope, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Check the implementer behind one erasure against the shape its constraint registered.
    pub(in crate::lower) fn check_erasure(
        &mut self,
        tree: &mut mir::Tree,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        scope: &GenericScope,
    ) -> CompilerResult<()> {
        // read the constraint from the erased target
        let lifetimes = scope.erased();
        let dynamic = self.type_lowerer(tree, &lifetimes).lower(target)?;
        let mir::Type::Dynamic { constraint, .. } = *tree.get(dynamic) else {
            return Err(CompilerError::Internal {
                message: "a value erased outside a dynamic target".to_string(),
            });
        };

        // classify the erased source beneath its forms, leaving an open one to its specialization
        let mut source = self.ty(source)?;
        while let dir::Type::Form(form) = source {
            source = self.ty(form.value)?;
        }
        if matches!(source, dir::Type::Parameter(_)) {
            return Ok(());
        }
        let is_object = matches!(source, dir::Type::Object(_));
        let is_class = matches!(source, dir::Type::Application(_));

        // read the dispatch shape the constraint registered
        let Some((shape, _)) = self.dynamic_shape(tree, constraint) else {
            return Err(CompilerError::Internal {
                message: "an erasure without a registered constraint shape".to_string(),
            });
        };

        // reject the slots no implementer can answer
        for slot in &shape.slots {
            let construct = match slot {
                mir::DynamicSlot::Function { name: None, .. } => {
                    "a call-signature constraint member"
                }
                mir::DynamicSlot::Function { .. } if is_object => {
                    "a function member on a structural constraint"
                }
                mir::DynamicSlot::Function { .. } | mir::DynamicSlot::Field { .. } => continue,
            };

            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: construct.to_string(),
            }
            .into());
        }

        // reject a structural value erased at a constraint with slots
        if !is_object && !is_class && !shape.slots.is_empty() {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "erasing a structural value".to_string(),
            }
            .into());
        }

        Ok(())
    }

    /// Publish the dispatch shapes the lowered constraints registered.
    pub(in crate::lower) fn publish_dispatch_shapes(&mut self, builder: &mut mir::ModuleBuilder) {
        for (_, shape) in mem::take(&mut self.dynamic_shapes) {
            builder.dispatch_mut().insert_dynamic_shape(shape);
        }
    }
}
