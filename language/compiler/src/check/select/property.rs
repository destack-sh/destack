use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Decision, Dependency, MemberLookup, Origin, Relation, answer,
};

/// One literal property entry collected for merging.
enum MergeEntry {
    /// One direct keyed field or method.
    Field {
        /// The property key.
        key: dir::StaticKey,
        /// The typed source node.
        source: dir::GlobalNodeIdAny,
    },
    /// One spread source.
    Spread {
        /// The spread value node.
        source: dir::GlobalNodeIdAny,
    },
}

impl CheckState<'_> {
    /// Select the merged shape of one literal with spread properties.
    pub(in crate::check) fn select_property_merge(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        properties: &[dir::LocalNodeId<dir::Property>],
        target: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let origin = Origin::Node(node.into_any());

        // collect the property entries in source order
        let view = self.module(module).view();
        let mut entries = SmallVec::<[MergeEntry; 8]>::new();
        for property in properties {
            match view.get(*property) {
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = key.direct_static_key() else {
                        continue;
                    };

                    entries.push(MergeEntry::Field {
                        key,
                        source: value.into_global_any(module),
                    });
                }
                dir::Property::Method { key, .. } => {
                    let Some(key) = key.and_then(dir::Key::direct_static_key) else {
                        continue;
                    };

                    entries.push(MergeEntry::Field {
                        key,
                        source: property.into_global_any(module),
                    });
                }
                dir::Property::Spread { value } => entries.push(MergeEntry::Spread {
                    source: value.into_global_any(module),
                }),
                dir::Property::Error => {}
            }
        }

        // merge entry fields left to right, later keys overriding
        let mut fields = IndexMap::<dir::StaticKey, dir::TypeField>::new();
        for entry in entries {
            match entry {
                // direct fields read their walked node or method type
                MergeEntry::Field { key, source } => {
                    let ty = answer!(self.merge_entry_type(source)?);
                    fields.insert(
                        key,
                        dir::TypeField {
                            key,
                            ty,
                            is_optional: false,
                            is_readonly: false,
                        },
                    );
                }
                // spread sources contribute every visible field
                MergeEntry::Spread { source } => {
                    let spread = answer!(self.node_type_answer(source)?);
                    let Some(spread_fields) = answer!(self.spread_fields(origin, module, spread)?)
                    else {
                        // unspreadable sources reject the literal loudly
                        return self.reject_spread(node, source, spread);
                    };
                    for field in spread_fields {
                        fields.insert(field.key, field);
                    }
                }
            }
        }

        // merge the shape at the literal node
        let shape = self.push_type(
            module,
            dir::Type::Shape(dir::ShapeType {
                fields: fields.into_values().collect(),
                call_signatures: Vec::new(),
                construct_signatures: Vec::new(),
                index_signatures: Vec::new(),
            }),
            node.local_id.into_any(),
        )?;

        match target {
            // struct literals must fill their declared fields
            Some(target) => self.relate(origin, Relation::Writable, None, shape, target),
            // object literals bind their managed merged shape
            None => {
                let managed = self.push_type(
                    module,
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Managed,
                        value: shape,
                    }),
                    node.local_id.into_any(),
                )?;
                self.bind_node_type(node.into_any(), managed)?;

                Ok(Answer::Ready(()))
            }
        }
    }

    /// Return the merged field type at one entry source node.
    fn merge_entry_type(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // method properties type through their declaration symbol
        if source.local_id.ty == dir::NodeType::Property
            && let Some(symbol) = self
                .module(source.module_id)
                .declaration_symbol(source.local_id)
            && let Some(ty) = self.symbol_type_maybe(symbol)
        {
            return Ok(Answer::Ready(ty));
        }

        self.node_type_answer(source)
    }

    /// Return the mergeable fields of one spread source.
    fn spread_fields(
        &mut self,
        origin: Origin,
        module: ModuleId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<Vec<dir::TypeField>>>> {
        // close the spread source first
        let root = answer!(self.reduce_type_root(origin, ty)?);
        if let Some(variable) = self.root_variable(root)? {
            return Ok(Answer::pending([Dependency::Variable(variable)]));
        }

        // peel managed forms to the carried value
        let mut current = root;
        while let dir::Type::Form(form) = self.ty(current)? {
            if form.form != dir::Form::Managed {
                break;
            }
            current = self.settled_root(form.value)?;
        }

        match self.ty(current)?.clone() {
            // structural shapes spread their fields directly
            dir::Type::Shape(shape) => Ok(Answer::Ready(Some(shape.fields))),
            // instances spread their visible fields
            dir::Type::Instance(instance) => {
                let keys = self.nominal_member_keys(instance.symbol);
                let mut fields = Vec::with_capacity(keys.len());
                let mut seen = SmallVec::<[dir::StaticKey; 8]>::new();
                for key in keys {
                    if seen.contains(&key) {
                        continue;
                    }
                    seen.push(key);

                    // read each member through the receiver instance
                    let lookup = self.lookup_member(
                        origin,
                        module,
                        current,
                        dir::MemberSpace::Instance,
                        key,
                    )?;
                    let ty = match lookup {
                        MemberLookup::Field(ty) => Some(ty),
                        MemberLookup::Found(candidates) => {
                            candidates.first().map(|candidate| candidate.ty)
                        }
                        MemberLookup::Missing => None,
                        MemberLookup::Pending(blockers) => {
                            return Ok(Answer::Pending(blockers));
                        }
                    };
                    if let Some(ty) = ty {
                        fields.push(dir::TypeField {
                            key,
                            ty,
                            is_optional: false,
                            is_readonly: false,
                        });
                    }
                }

                Ok(Answer::Ready(Some(fields)))
            }
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Reject one literal whose spread source has no fields.
    fn reject_spread(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        source: dir::GlobalNodeIdAny,
        spread: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        self.report_spread_not_object(Origin::Node(source), spread)?;
        self.record_decision(node.into_any(), Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }
}
