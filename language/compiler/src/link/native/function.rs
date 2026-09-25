use tspp_native as native;

use crate::LinkResult;

use super::{Image, NativeLinker};

impl<'a, 'b> NativeLinker<'a, 'b> {
    /// Link native entries in dense Program function order.
    pub(super) fn link_functions(
        &self,
        image: &Image,
    ) -> LinkResult<Vec<Option<native::Function>>> {
        let mut functions = Vec::with_capacity(self.program.functions_by_id().len());

        for &(module, function) in self.program.functions_by_id() {
            let object = self.program.object(module);
            let source = self.source(object)?;
            let index = object
                .functions()
                .binary_search_by_key(&function, |declaration| declaration.id)
                .map_err(|_| {
                    self.program
                        .invalid_input("native function declaration is absent")
                })?;
            let definition = source
                .definitions()
                .get(index)
                .and_then(|definition| definition.get());
            let Some(definition) = definition else {
                functions.push(None);
                continue;
            };
            let body = native::Entry::new(self.block(image, module, definition.body)?);
            let entry = native::Entry::new(self.block(image, module, definition.entry)?);
            functions.push(Some(native::Function::new(body, entry)));
        }

        Ok(functions)
    }

    /// Link coroutine resume entries in canonical frame-state order.
    pub(super) fn link_resumes(&self, image: &Image) -> LinkResult<Vec<Option<native::Entry>>> {
        let mut resumes = vec![None; self.frames.len()];

        for (module, object) in self.program.objects() {
            let source = self.source(object)?;
            for definition in source.definitions().iter().filter_map(|entry| entry.get()) {
                for resume in definition.resumes(source.resumes()) {
                    let source_state =
                        object.frames().get(resume.state as usize).ok_or_else(|| {
                            self.program
                                .invalid_input("native resume references an unknown frame state")
                        })?;
                    let state =
                        self.frames
                            .state(*module, source_state.point)
                            .ok_or_else(|| {
                                self.program
                                    .invalid_input("native resume state was not linked")
                            })?;
                    let entry = native::Entry::new(self.block(image, *module, resume.block)?);
                    if resumes[state.index()].replace(entry).is_some() {
                        return Err(self.program.invalid_input("native resume is defined twice"));
                    }
                }
            }
        }

        Ok(resumes)
    }
}
