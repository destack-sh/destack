use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::CheckState;

impl CheckState<'_> {
    /// Return whether one pattern field list uses rest fields correctly.
    pub(in crate::check) fn check_pattern_rest_fields(
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

        self.check_rest_fields(module, fields)
    }

    /// Return whether one assignment pattern field list uses rest fields correctly.
    pub(in crate::check) fn check_assign_pattern_rest_fields(
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

        self.check_rest_fields(module, fields)
    }

    /// Check static keys in one object pattern field list.
    pub(in crate::check) fn check_pattern_field_keys(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<()> {
        let keys = fields
            .iter()
            .map(|field| self.pattern_field_key(module, *field))
            .collect::<CompilerResult<Vec<_>>>()?;

        self.check_field_keys(module, keys.into_iter().flatten())
    }

    /// Check static keys in one object assignment pattern field list.
    pub(in crate::check) fn check_assign_pattern_field_keys(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> CompilerResult<()> {
        let keys = fields
            .iter()
            .map(|field| self.assign_pattern_field_key(module, *field))
            .collect::<CompilerResult<Vec<_>>>()?;

        self.check_field_keys(module, keys.into_iter().flatten())
    }

    /// Check direct binding names in one pattern field list.
    pub(in crate::check) fn check_pattern_bindings(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<()> {
        let mut names = SmallVec::<[(dir::StringId, dir::LocalNodeIdAny); 8]>::new();
        for field in fields {
            self.check_pattern_field_bindings(module, *field, &mut names)?;
        }

        Ok(())
    }

    /// Return the static key and diagnostic source of one pattern field.
    fn pattern_field_key(
        &self,
        module: ModuleId,
        field: dir::LocalNodeId<dir::PatternField>,
    ) -> CompilerResult<Option<(dir::LocalNodeIdAny, dir::StaticKey)>> {
        let key = match self.module(module).view().get(field).clone() {
            dir::PatternField::Named { name, .. } => Some((field.into_any(), name.static_key())),
            dir::PatternField::Computed { key, .. } => self
                .static_key_from_expression(module, key)?
                .map(|static_key| (key.into_any(), static_key)),
            dir::PatternField::Rest { .. }
            | dir::PatternField::Elision
            | dir::PatternField::Positional { .. } => None,
        };

        Ok(key)
    }

    /// Return the static key and diagnostic source of one assignment pattern field.
    fn assign_pattern_field_key(
        &self,
        module: ModuleId,
        field: dir::LocalNodeId<dir::AssignPatternField>,
    ) -> CompilerResult<Option<(dir::LocalNodeIdAny, dir::StaticKey)>> {
        let key = match self.module(module).view().get(field).clone() {
            dir::AssignPatternField::Named { name, .. } => {
                Some((field.into_any(), name.static_key()))
            }
            dir::AssignPatternField::Computed { key, .. } => self
                .static_key_from_expression(module, key)?
                .map(|static_key| (key.into_any(), static_key)),
            dir::AssignPatternField::Rest { .. }
            | dir::AssignPatternField::Elision
            | dir::AssignPatternField::Positional { .. } => None,
        };

        Ok(key)
    }

    /// Check binding names introduced directly by one pattern field.
    fn check_pattern_field_bindings(
        &mut self,
        module: ModuleId,
        field: dir::LocalNodeId<dir::PatternField>,
        names: &mut SmallVec<[(dir::StringId, dir::LocalNodeIdAny); 8]>,
    ) -> CompilerResult<()> {
        let source = field.into_any();
        let field_node = self.module(module).view().get(field).clone();
        match field_node {
            // shorthand fields introduce their field name
            dir::PatternField::Named {
                name,
                pattern: None,
                ..
            } => {
                if let dir::Name::Identifier(name) | dir::Name::String(name) = name {
                    self.check_pattern_binding_name(module, name, source, names)?;
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
                self.check_pattern_binding(module, pattern, names)
            }

            // rest fields only introduce names through an explicit nested pattern
            dir::PatternField::Rest { pattern } => {
                if let Some(pattern) = pattern {
                    self.check_pattern_binding(module, pattern, names)?;
                }

                Ok(())
            }

            // elisions introduce no binding
            dir::PatternField::Elision => Ok(()),
        }
    }

    /// Check one object field key stream for duplicate keys.
    fn check_field_keys<I>(&mut self, module: ModuleId, keys: I) -> CompilerResult<()>
    where
        I: IntoIterator<Item = (dir::LocalNodeIdAny, dir::StaticKey)>,
    {
        let mut seen = SmallVec::<[(dir::StaticKey, dir::LocalNodeIdAny); 8]>::new();
        for (source, key) in keys {
            // report every repeated field at its repeated key
            if seen.iter().any(|(existing, _)| *existing == key) {
                let key = self.format_static_key(&key);
                self.report_duplicate_pattern_field(module, source, key);
            } else {
                seen.push((key, source));
            }
        }

        Ok(())
    }

    /// Check one direct binding name.
    fn check_pattern_binding_name(
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

    /// Check binding names introduced by one direct field pattern.
    fn check_pattern_binding(
        &mut self,
        module: ModuleId,
        pattern: dir::LocalNodeId<dir::Pattern>,
        names: &mut SmallVec<[(dir::StringId, dir::LocalNodeIdAny); 8]>,
    ) -> CompilerResult<()> {
        let source = pattern.into_any();
        let pattern = self.module(module).view().get(pattern).clone();
        match pattern {
            // leaf bindings introduce one name into the current field list
            dir::Pattern::Binding { name, pattern } => {
                self.check_pattern_binding_name(module, name, source, names)?;
                if let Some(pattern) = pattern {
                    self.check_pattern_binding(module, pattern, names)?;
                }
            }

            // transparent patterns do not create independent field lists
            dir::Pattern::Must(pattern)
            | dir::Pattern::BorrowOf { right: pattern, .. }
            | dir::Pattern::MoveOf { right: pattern, .. }
            | dir::Pattern::DereferenceOf { right: pattern } => {
                self.check_pattern_binding(module, pattern, names)?;
            }
            dir::Pattern::Default { pattern, .. } => {
                self.check_pattern_binding(module, pattern, names)?;
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

    /// Return whether one field list uses rest fields correctly.
    fn check_rest_fields<I>(&mut self, module: ModuleId, fields: I) -> bool
    where
        I: IntoIterator<Item = (dir::LocalNodeIdAny, bool)>,
    {
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
