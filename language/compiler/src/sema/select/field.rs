use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::CompilerResult;
use crate::sema::CheckState;

impl CheckState<'_> {
    /// Return whether one pattern field list places its rest field last.
    pub(in crate::sema) fn report_pattern_rest_fields(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> bool {
        let fields = fields
            .iter()
            .map(|field| {
                let is_rest = matches!(
                    self.module(module).view().get(*field),
                    dir::PatternField::Rest { .. }
                );

                (field.into_any(), is_rest)
            })
            .collect::<SmallVec<[_; 8]>>();

        self.report_rest_fields(module, fields)
    }

    /// Return whether one assignment pattern field list places its rest field last.
    pub(in crate::sema) fn report_assign_rest_fields(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> bool {
        let fields = fields
            .iter()
            .map(|field| {
                let is_rest = matches!(
                    self.module(module).view().get(*field),
                    dir::AssignPatternField::Rest { .. }
                );

                (field.into_any(), is_rest)
            })
            .collect::<SmallVec<[_; 8]>>();

        self.report_rest_fields(module, fields)
    }

    /// Report every repeated key in one pattern field list.
    pub(in crate::sema) fn report_duplicate_pattern_fields(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<()> {
        let keys = fields
            .iter()
            .map(|field| self.pattern_field_key(module, *field))
            .collect::<CompilerResult<Vec<_>>>()?;

        self.report_duplicate_field_keys(module, keys.into_iter().flatten())
    }

    /// Report every repeated key in one assignment pattern field list.
    pub(in crate::sema) fn report_duplicate_assign_fields(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> CompilerResult<()> {
        let keys = fields
            .iter()
            .map(|field| self.assign_pattern_field_key(module, *field))
            .collect::<CompilerResult<Vec<_>>>()?;

        self.report_duplicate_field_keys(module, keys.into_iter().flatten())
    }

    /// Report every repeated binding name in one pattern field list.
    pub(in crate::sema) fn report_duplicate_pattern_bindings(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<()> {
        let mut names = SmallVec::<[(dir::StringId, dir::LocalNodeIdAny); 8]>::new();
        for field in fields {
            self.report_duplicate_field_binding(module, *field, &mut names)?;
        }

        Ok(())
    }

    /// Return the static key and diagnostic source of one pattern field.
    fn pattern_field_key(
        &mut self,
        module: ModuleId,
        field: dir::LocalNodeId<dir::PatternField>,
    ) -> CompilerResult<Option<(dir::LocalNodeIdAny, dir::StaticKey)>> {
        // read the key each field form names
        let key = match self.module(module).view().get(field).clone() {
            dir::PatternField::Named { name, .. } => Some((field.into_any(), name.into())),
            dir::PatternField::Computed { key, .. } => self
                .evaluate_static_key(module, key)?
                .map(|static_key| (key.into_any(), static_key)),
            dir::PatternField::Rest { .. }
            | dir::PatternField::Elision
            | dir::PatternField::Positional { .. } => None,
        };

        Ok(key)
    }

    /// Return the static key and diagnostic source of one assignment pattern field.
    fn assign_pattern_field_key(
        &mut self,
        module: ModuleId,
        field: dir::LocalNodeId<dir::AssignPatternField>,
    ) -> CompilerResult<Option<(dir::LocalNodeIdAny, dir::StaticKey)>> {
        // read the key each field form names
        let key = match self.module(module).view().get(field).clone() {
            dir::AssignPatternField::Named { name, .. } => Some((field.into_any(), name.into())),
            dir::AssignPatternField::Computed { key, .. } => self
                .evaluate_static_key(module, key)?
                .map(|static_key| (key.into_any(), static_key)),
            dir::AssignPatternField::Rest { .. }
            | dir::AssignPatternField::Elision
            | dir::AssignPatternField::Positional { .. } => None,
        };

        Ok(key)
    }

    /// Report every repeated key in one field key stream.
    fn report_duplicate_field_keys<I>(&mut self, module: ModuleId, keys: I) -> CompilerResult<()>
    where
        I: IntoIterator<Item = (dir::LocalNodeIdAny, dir::StaticKey)>,
    {
        // report every repeated field at its repeated key
        let mut seen = SmallVec::<[(dir::StaticKey, dir::LocalNodeIdAny); 8]>::new();
        for (source, key) in keys {
            if let Some((_, first)) = seen.iter().find(|(existing, _)| *existing == key) {
                let key = self.format_static_key(&key);
                self.report_duplicate_pattern_field(module, source, *first, key);
            } else {
                seen.push((key, source));
            }
        }

        Ok(())
    }

    /// Report every repeated binding name introduced directly by one pattern field.
    fn report_duplicate_field_binding(
        &mut self,
        module: ModuleId,
        field: dir::LocalNodeId<dir::PatternField>,
        names: &mut SmallVec<[(dir::StringId, dir::LocalNodeIdAny); 8]>,
    ) -> CompilerResult<()> {
        let source = field.into_any();
        let field_node = self.module(module).view().get(field).clone();
        // introduce the names each field form binds
        match field_node {
            // shorthand fields introduce their field name
            dir::PatternField::Named {
                name,
                pattern: None,
                ..
            } => {
                if let dir::Name::Identifier(name) | dir::Name::String(name) = name {
                    self.report_duplicate_binding_name(module, name, source, names)?;
                }

                Ok(())
            }

            // explicit fields introduce names through their nested pattern
            dir::PatternField::Named {
                pattern: Some(pattern),
                ..
            }
            | dir::PatternField::Computed { pattern, .. }
            | dir::PatternField::Positional { pattern } => {
                self.report_duplicate_bindings(module, pattern, names)
            }

            // rest fields only introduce names through an explicit nested pattern
            dir::PatternField::Rest { pattern } => {
                if let Some(pattern) = pattern {
                    self.report_duplicate_bindings(module, pattern, names)?;
                }

                Ok(())
            }

            // elisions introduce no binding
            dir::PatternField::Elision => Ok(()),
        }
    }

    /// Report one binding name already introduced by this field list.
    fn report_duplicate_binding_name(
        &mut self,
        module: ModuleId,
        name: dir::StringId,
        source: dir::LocalNodeIdAny,
        names: &mut SmallVec<[(dir::StringId, dir::LocalNodeIdAny); 8]>,
    ) -> CompilerResult<()> {
        // report every repeated binding at its repeated name
        if names.iter().any(|(existing, _)| *existing == name) {
            let name = self.format_static_key(&dir::StaticKey::Name(name));
            self.report_duplicate_pattern_binding(module, source, name);
        } else {
            names.push((name, source));
        }

        Ok(())
    }

    /// Report every repeated binding name introduced by one field pattern.
    fn report_duplicate_bindings(
        &mut self,
        module: ModuleId,
        pattern: dir::LocalNodeId<dir::Pattern>,
        names: &mut SmallVec<[(dir::StringId, dir::LocalNodeIdAny); 8]>,
    ) -> CompilerResult<()> {
        let source = pattern.into_any();
        let pattern = self.module(module).view().get(pattern).clone();
        // introduce the names each pattern form binds
        match pattern {
            // leaf bindings introduce one name into the current field list
            dir::Pattern::Binding { name, pattern } => {
                self.report_duplicate_binding_name(module, name, source, names)?;
                if let Some(pattern) = pattern {
                    self.report_duplicate_bindings(module, pattern, names)?;
                }
            }

            // transparent patterns share the enclosing field list
            dir::Pattern::Must(pattern)
            | dir::Pattern::BorrowOf { right: pattern, .. }
            | dir::Pattern::MoveOf { right: pattern, .. }
            | dir::Pattern::DereferenceOf { right: pattern } => {
                self.report_duplicate_bindings(module, pattern, names)?;
            }
            dir::Pattern::Default { pattern, .. } => {
                self.report_duplicate_bindings(module, pattern, names)?;
            }

            // nested destructures own their own immediate binding checks
            dir::Pattern::Tuple { .. }
            | dir::Pattern::NominalTuple { .. }
            | dir::Pattern::Sequence { .. }
            | dir::Pattern::Object { .. }
            | dir::Pattern::NominalObject { .. }
            | dir::Pattern::Union { .. }
            | dir::Pattern::Wildcard
            | dir::Pattern::Expression { .. }
            | dir::Pattern::Range { .. } => {}
        }

        Ok(())
    }

    /// Return whether one field list places a single rest field last.
    fn report_rest_fields<I>(&mut self, module: ModuleId, fields: I) -> bool
    where
        I: IntoIterator<Item = (dir::LocalNodeIdAny, bool)>,
    {
        // locate the rest fields among the written positions
        let fields = fields.into_iter().collect::<SmallVec<[_; 8]>>();
        let rest_fields = fields
            .iter()
            .enumerate()
            .filter_map(|(position, (source, is_rest))| is_rest.then_some((position, *source)))
            .collect::<SmallVec<[_; 2]>>();

        // reject extra rests before checking terminal position
        if rest_fields.len() > 1 {
            for (_, source) in rest_fields.iter().skip(1) {
                self.report_multiple_rest_patterns(module, *source);
            }

            return false;
        }

        // reject a single rest followed by any field
        let Some((position, source)) = rest_fields.first().copied() else {
            return true;
        };
        if position + 1 < fields.len() {
            self.report_rest_pattern_not_last(module, source);

            return false;
        }

        true
    }
}
