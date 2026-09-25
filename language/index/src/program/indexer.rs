use tspp_artifact::{IndexKind, ModuleIndex, ProgramIndex};
use tspp_dir as dir;
use tspp_repository::{ProviderError, ProviderResult};

/// Builder for one program index from matching module indexes.
pub(crate) struct ProgramIndexer<'a> {
    /// The current module indexes.
    pub(crate) modules: Vec<&'a ModuleIndex>,
}

impl ProgramIndexer<'_> {
    /// Build one program index.
    pub(crate) fn build(&self, kind: IndexKind) -> ProviderResult<ProgramIndex> {
        let index = match kind {
            IndexKind::Symbols => ProgramIndex::Symbols(dir::SymbolPostings::build(
                &self.indexes(kind, |module| match module {
                    ModuleIndex::Symbols(index) => Some(index),
                    _ => None,
                })?,
            )),
            IndexKind::Exports => ProgramIndex::Exports(dir::ExportPostings::build(
                &self.indexes(kind, |module| match module {
                    ModuleIndex::Exports(index) => Some(index),
                    _ => None,
                })?,
            )),
            IndexKind::Members => ProgramIndex::Members(dir::MemberPostings::build(
                &self.indexes(kind, |module| match module {
                    ModuleIndex::Members(index) => Some(index),
                    _ => None,
                })?,
            )),
            IndexKind::References => ProgramIndex::References(dir::ReferencePostings::build(
                &self.indexes(kind, |module| match module {
                    ModuleIndex::References(index) => Some(index),
                    _ => None,
                })?,
            )),
            IndexKind::Calls => {
                ProgramIndex::Calls(dir::CallPostings::build(&self.indexes(kind, |module| {
                    match module {
                        ModuleIndex::Calls(index) => Some(index),
                        _ => None,
                    }
                })?))
            }
            IndexKind::Heritage => ProgramIndex::Heritage(dir::HeritagePostings::build(
                &self.indexes(kind, |module| match module {
                    ModuleIndex::Heritage(index) => Some(index),
                    _ => None,
                })?,
            )),
            IndexKind::Decorators => ProgramIndex::Decorators(dir::DecoratorPostings::build(
                &self.indexes(kind, |module| match module {
                    ModuleIndex::Decorators(index) => Some(index),
                    _ => None,
                })?,
            )),
            IndexKind::Code => {
                ProgramIndex::Code(dir::CodePostings::build(&self.indexes(kind, |module| {
                    match module {
                        ModuleIndex::Code(index) => Some(index),
                        _ => None,
                    }
                })?))
            }
        };

        Ok(index)
    }

    /// Update one program index from changed module indexes.
    pub(crate) fn update(
        mut index: ProgramIndex,
        modules: &[(u32, &ModuleIndex)],
    ) -> ProviderResult<ProgramIndex> {
        for (ordinal, module) in modules {
            match (&mut index, *module) {
                (ProgramIndex::Symbols(postings), ModuleIndex::Symbols(index)) => {
                    postings.update(*ordinal, index);
                }
                (ProgramIndex::Exports(postings), ModuleIndex::Exports(index)) => {
                    postings.update(*ordinal, index);
                }
                (ProgramIndex::Members(postings), ModuleIndex::Members(index)) => {
                    postings.update(*ordinal, index);
                }
                (ProgramIndex::References(postings), ModuleIndex::References(index)) => {
                    postings.update(*ordinal, index);
                }
                (ProgramIndex::Calls(postings), ModuleIndex::Calls(index)) => {
                    postings.update(*ordinal, index);
                }
                (ProgramIndex::Heritage(postings), ModuleIndex::Heritage(index)) => {
                    postings.update(*ordinal, index);
                }
                (ProgramIndex::Decorators(postings), ModuleIndex::Decorators(index)) => {
                    postings.update(*ordinal, index);
                }
                (ProgramIndex::Code(postings), ModuleIndex::Code(index)) => {
                    postings.update(*ordinal, index);
                }
                (program, module) => {
                    return Err(ProviderError::internal(format!(
                        "program {:?} index received module {:?}",
                        program.kind(),
                        module.kind()
                    ))
                    .into());
                }
            }
        }

        Ok(index)
    }

    /// Return one matching index from every module.
    fn indexes<'a, Index>(
        &'a self,
        kind: IndexKind,
        select: impl Fn(&'a ModuleIndex) -> Option<&'a Index>,
    ) -> ProviderResult<Vec<&'a Index>> {
        let mut indexes = Vec::with_capacity(self.modules.len());

        // require one matching family from every module
        for module in &self.modules {
            let index = select(module).ok_or_else(|| {
                ProviderError::internal(format!(
                    "program {kind:?} index received module {:?}",
                    module.kind()
                ))
            })?;
            indexes.push(index);
        }

        Ok(indexes)
    }
}
