use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Decision, Dependency, FlowSite, Origin, PlaceUse, Relation, answer,
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
        site: FlowSite,
        properties: &[dir::LocalNodeId<dir::Property>],
        target: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let origin = Origin::Node(node.into_any());

        // collect the property entries in source order
        let mut entries = SmallVec::<[MergeEntry; 8]>::new();
        for property in properties {
            match self.module(module).view().get(*property).clone() {
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = answer!(self.select_property_key(site, key)?) else {
                        continue;
                    };

                    entries.push(MergeEntry::Field {
                        key,
                        source: value.into_global_any(module),
                    });
                }
                dir::Property::Method { key, .. } => {
                    let Some(key) = answer!(match key {
                        Some(key) => self.select_property_key(site, key)?,
                        None => Answer::Ready(None),
                    }) else {
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
                // direct fields read their inferred node or method type
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
                    let source_site = self.node_site(source)?;
                    let spread = answer!(self.infer_node_type(source_site, PlaceUse::Read)?);
                    let Some(spread_fields) = answer!(self.spread_fields(origin, module, spread)?)
                    else {
                        // reject spreads with no field projection
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
            Some(target) => {
                let () = answer!(self.relate(origin, Relation::Writable, None, shape, target)?);
                self.commit_node_type(node.into_any(), target)?;

                Ok(Answer::Ready(()))
            }
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
                self.commit_node_type(node.into_any(), managed)?;

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
        {
            return match self.symbol_type_maybe(symbol) {
                Some(ty) => Ok(Answer::Ready(ty)),
                None => Ok(Answer::pending([Dependency::SymbolType(symbol)])),
            };
        }

        let source_site = self.node_site(source)?;
        self.infer_node_type(source_site, PlaceUse::Read)
    }

    /// Return the mergeable fields of one spread source.
    pub(in crate::check) fn spread_fields(
        &mut self,
        origin: Origin,
        module: ModuleId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<Vec<dir::TypeField>>>> {
        // reduce the spread source before merging fields
        let root = answer!(self.reduce_type_head(origin, ty)?);
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
                    let lookup = answer!(self.lookup_member(
                        origin,
                        module,
                        current,
                        dir::MemberSpace::Instance,
                        key,
                    )?);
                    let ty = lookup.value_type();
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
        self.commit_decision(node.into_any(), Decision::Rejected)?;
        self.commit_error_node(node.into_any())?;

        Ok(Answer::Ready(()))
    }
}
