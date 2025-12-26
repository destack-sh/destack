use crate::{AnalyzeError, AnalyzeResult, Assignability, Compiler};
use destack_builtin::LanguageItem;
use destack_dir::{
    BinaryOperator, DeclarationType, Expression, Extension, ExtensionKind, GlobalSymbolId, IntType,
    LocalNodeId, LocalTypeId, Mutability, PrimitiveType, ScalarLiteral, StaticKey, SymbolType,
    Type, TypeBinaryOperator, TypeField, TypeLiteral, TypeTable, TypeUnaryOperator, UnaryOperator,
    VarianceBound,
};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Infer the result type of a scalar literal.
    /// Returns the literal type (e.g., `4` has type `4`), allowing assignability to check
    /// whether the literal fits the target type (int32, number, etc.).
    pub(super) fn infer_scalar_literal(&self, value: &ScalarLiteral) -> TypeLiteral {
        // return the literal type, not the widened primitive type
        // this allows `let x: int = 4` to work via assignability checking
        TypeLiteral::ScalarLiteral(value.clone())
    }

    /// Infer the result type of a binary operation.
    pub(super) fn infer_binary_operation(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
        types: &TypeTable,
    ) -> Type {
        match operator {
            // comparison operators: try constant folding, else return boolean
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual => self
                .try_fold_comparison(operator, left, right)
                .unwrap_or(Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                }),

            // in/instanceof always return boolean (no constant folding)
            BinaryOperator::In | BinaryOperator::InstanceOf => Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            },

            // logical operators: try constant folding, else return boolean
            BinaryOperator::And | BinaryOperator::Or => self
                .try_fold_logical(operator, left, right)
                .unwrap_or(Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                }),

            // arithmetic operators: try constant folding, else widen types
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder
            | BinaryOperator::Exponent => self
                .try_infer_string_concatenation(operator, left, right, types)
                .or_else(|| self.try_fold_arithmetic(operator, left, right))
                .unwrap_or_else(|| self.widen_numeric_types(left, right)),

            _ => left.clone(),
        }
    }

    /// Try to constant fold a comparison operation on literal types.
    fn try_fold_comparison(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
    ) -> Option<Type> {
        // extract scalar literals from both sides
        let (left_lit, right_lit) = Self::extract_scalar_literals(left, right)?;

        // compare integers
        if let (ScalarLiteral::Integer(left_value), ScalarLiteral::Integer(right_value)) =
            (left_lit, right_lit)
        {
            let result = match operator {
                BinaryOperator::Equal | BinaryOperator::EqualStrict => left_value == right_value,
                BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict => {
                    left_value != right_value
                }
                BinaryOperator::LessThan => left_value < right_value,
                BinaryOperator::LessThanOrEqual => left_value <= right_value,
                BinaryOperator::GreaterThan => left_value > right_value,
                BinaryOperator::GreaterThanOrEqual => left_value >= right_value,
                _ => return None,
            };
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(result)),
            });
        }

        // compare floats (or mixed int/float)
        let (left_value, right_value) = Self::to_f64_pair(left_lit, right_lit)?;
        let result = match operator {
            BinaryOperator::Equal | BinaryOperator::EqualStrict => left_value == right_value,
            BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict => left_value != right_value,
            BinaryOperator::LessThan => left_value < right_value,
            BinaryOperator::LessThanOrEqual => left_value <= right_value,
            BinaryOperator::GreaterThan => left_value > right_value,
            BinaryOperator::GreaterThanOrEqual => left_value >= right_value,
            _ => return None,
        };
        Some(Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(result)),
        })
    }

    /// Try to constant fold a logical operation on literal types.
    fn try_fold_logical(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
    ) -> Option<Type> {
        // extract boolean literals from both sides
        let (left_lit, right_lit) = Self::extract_scalar_literals(left, right)?;

        if let (ScalarLiteral::Boolean(left_value), ScalarLiteral::Boolean(right_value)) =
            (left_lit, right_lit)
        {
            let result = match operator {
                BinaryOperator::And => *left_value && *right_value,
                BinaryOperator::Or => *left_value || *right_value,
                _ => return None,
            };
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(result)),
            });
        }

        None
    }

    /// Try to constant fold an arithmetic operation on literal types.
    fn try_fold_arithmetic(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
    ) -> Option<Type> {
        // extract scalar literals from both sides
        let (left_lit, right_lit) = Self::extract_scalar_literals(left, right)?;

        // try to fold integer operations (preserves integer type)
        if let (ScalarLiteral::Integer(left_value), ScalarLiteral::Integer(right_value)) =
            (left_lit, right_lit)
        {
            let result = match operator {
                BinaryOperator::Add => left_value.checked_add(*right_value),
                BinaryOperator::Subtract => left_value.checked_sub(*right_value),
                BinaryOperator::Multiply => left_value.checked_mul(*right_value),
                BinaryOperator::Divide => {
                    if *right_value != 0 {
                        left_value.checked_div(*right_value)
                    } else {
                        None
                    }
                }
                BinaryOperator::Remainder => {
                    if *right_value != 0 {
                        left_value.checked_rem(*right_value)
                    } else {
                        None
                    }
                }
                BinaryOperator::Exponent => {
                    if *right_value >= 0 && *right_value <= u32::MAX as i64 {
                        left_value.checked_pow(*right_value as u32)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(result) = result {
                return Some(Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(result)),
                });
            }
        }

        // try to fold float operations (if at least one operand is float)
        if matches!(left_lit, ScalarLiteral::Float(_))
            || matches!(right_lit, ScalarLiteral::Float(_))
        {
            let (left_value, right_value) = Self::to_f64_pair(left_lit, right_lit)?;
            let result = match operator {
                BinaryOperator::Add => left_value + right_value,
                BinaryOperator::Subtract => left_value - right_value,
                BinaryOperator::Multiply => left_value * right_value,
                BinaryOperator::Divide => left_value / right_value,
                BinaryOperator::Remainder => left_value % right_value,
                BinaryOperator::Exponent => left_value.powf(right_value),
                _ => return None,
            };
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(result)),
            });
        }

        None
    }

    /// Try to infer string concatenation for add.
    fn try_infer_string_concatenation(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
        types: &TypeTable,
    ) -> Option<Type> {
        if !matches!(operator, BinaryOperator::Add) {
            return None;
        }

        if self.is_string_like_type(left, types) || self.is_string_like_type(right, types) {
            return Some(Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            });
        }

        None
    }

    /// Check whether a type behaves like a string type.
    pub(super) fn is_string_like_type(&self, ty: &Type, types: &TypeTable) -> bool {
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            } => true,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_)),
            } => true,
            Type::Union { elements } => elements
                .iter()
                .all(|element_id| self.is_string_like_type(types.get_type(*element_id), types)),
            _ => false,
        }
    }

    /// Extract scalar literals from two types.
    fn extract_scalar_literals<'a>(
        left: &'a Type,
        right: &'a Type,
    ) -> Option<(&'a ScalarLiteral, &'a ScalarLiteral)> {
        match (left, right) {
            (
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(left_literal),
                },
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(right_literal),
                },
            ) => Some((left_literal, right_literal)),
            _ => None,
        }
    }

    /// Convert two scalar literals to f64 values (for numeric operations).
    fn to_f64_pair(left: &ScalarLiteral, right: &ScalarLiteral) -> Option<(f64, f64)> {
        let left_value = match left {
            ScalarLiteral::Integer(i) => *i as f64,
            ScalarLiteral::Float(f) => *f,
            _ => return None,
        };
        let right_value = match right {
            ScalarLiteral::Integer(i) => *i as f64,
            ScalarLiteral::Float(f) => *f,
            _ => return None,
        };
        Some((left_value, right_value))
    }

    /// Widen two numeric types to a common type.
    /// Used when constant folding fails (e.g., `x + 1` where x is a variable).
    fn widen_numeric_types(&self, left: &Type, right: &Type) -> Type {
        let left_prim = Self::to_numeric_primitive(left);
        let right_prim = Self::to_numeric_primitive(right);
        match (left_prim, right_prim) {
            // if both are known primitives, return the wider one
            (Some(left_primitive), Some(right_primitive)) => Type::TypeLiteral {
                value: TypeLiteral::Primitive(Self::wider_numeric_primitive(
                    &left_primitive,
                    &right_primitive,
                )),
            },
            // if one side is a primitive, use it
            (Some(p), None) | (None, Some(p)) => Type::TypeLiteral {
                value: TypeLiteral::Primitive(p),
            },
            // fallback to number
            (None, None) => Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            },
        }
    }

    /// Extract the numeric primitive type from a type.
    fn to_numeric_primitive(ty: &Type) -> Option<PrimitiveType> {
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(p),
            } if Self::is_numeric_primitive(p) => Some(*p),
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_)),
            } => Some(PrimitiveType::Number),
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(_)),
            } => Some(PrimitiveType::Number),
            _ => None,
        }
    }

    /// Check if a primitive type is numeric.
    fn is_numeric_primitive(p: &PrimitiveType) -> bool {
        matches!(
            p,
            PrimitiveType::Number
                | PrimitiveType::Int(_)
                | PrimitiveType::Float(_)
                | PrimitiveType::Bigint
        )
    }

    /// Return the wider of two numeric primitive types.
    fn wider_numeric_primitive(left: &PrimitiveType, right: &PrimitiveType) -> PrimitiveType {
        // number is the widest
        if matches!(left, PrimitiveType::Number) || matches!(right, PrimitiveType::Number) {
            return PrimitiveType::Number;
        }
        // float is wider than int; pick the wider float
        match (left, right) {
            (PrimitiveType::Float(left_float), PrimitiveType::Float(right_float)) => {
                let wider = if left_float.width() >= right_float.width() {
                    *left_float
                } else {
                    *right_float
                };
                return PrimitiveType::Float(wider);
            }
            (PrimitiveType::Float(f), _) | (_, PrimitiveType::Float(f)) => {
                return PrimitiveType::Float(*f);
            }
            _ => {}
        }
        // bigint stays bigint
        if matches!(left, PrimitiveType::Bigint) || matches!(right, PrimitiveType::Bigint) {
            return PrimitiveType::Bigint;
        }
        // compare int widths and return the wider one
        match (left, right) {
            (PrimitiveType::Int(left_int), PrimitiveType::Int(right_int)) => {
                // if either is signed, result should be signed
                let is_signed = left_int.is_signed() || right_int.is_signed();
                match (left_int.width(), right_int.width()) {
                    (Some(left_width), Some(right_width)) => {
                        let width = left_width.max(right_width);
                        PrimitiveType::Int(IntType::Arbitrary { width, is_signed })
                    }
                    // pointer sized ints: cannot determine width at compile time
                    _ => PrimitiveType::Number,
                }
            }
            (PrimitiveType::Int(i), _) | (_, PrimitiveType::Int(i)) => PrimitiveType::Int(*i),
            _ => PrimitiveType::Number,
        }
    }

    /// Infer the result type of a unary operation.
    pub(super) fn infer_unary_operation(&self, operator: &UnaryOperator, right: &Type) -> Type {
        match operator {
            // constant folding for logical not
            UnaryOperator::Not => self.try_fold_not(right).unwrap_or(Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            }),
            // constant folding for numeric negation
            UnaryOperator::Negate => self.try_fold_negate(right),
            _ => right.clone(),
        }
    }

    /// Try to constant fold logical not on a boolean literal.
    fn try_fold_not(&self, right: &Type) -> Option<Type> {
        if let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(b)),
        } = right
        {
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(!b)),
            });
        }
        None
    }

    /// Try to constant fold unary negation on a literal type.
    /// Returns the negated literal type, or the original type if folding is not possible.
    fn try_fold_negate(&self, right: &Type) -> Type {
        if let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(scalar),
        } = right
        {
            match scalar {
                ScalarLiteral::Integer(i) => {
                    if let Some(negated) = i.checked_neg() {
                        return Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(negated)),
                        };
                    }
                }
                ScalarLiteral::Float(f) => {
                    return Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(-f)),
                    };
                }
                _ => {}
            }
        }
        right.clone()
    }

    /// Infer the result type of a type unary operation.
    pub(super) fn infer_type_unary_operation(
        &self,
        _operator: &TypeUnaryOperator,
        _right_ty_id: LocalTypeId,
        _types: &TypeTable,
    ) -> Type {
        // NOTE #Incomplete: type level unary operation
        Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        }
    }

    /// Infer the result type of a type binary operation.
    pub(super) fn infer_type_binary_operation(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        operator: &TypeBinaryOperator,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Type {
        match operator {
            TypeBinaryOperator::Cast => {
                // type assertion: `x as T`
                // check if cast is valid (types overlap: at least one direction is assignable)
                let left_to_right = self.check_is_type_assignable(right_ty_id, left_ty_id, types);
                let right_to_left = self.check_is_type_assignable(left_ty_id, right_ty_id, types);

                if left_to_right == Assignability::NotAssignable
                    && right_to_left == Assignability::NotAssignable
                {
                    // neither direction works: illegal cast
                    self.error(AnalyzeError::InvalidCast {
                        node: expression_id.into_global_any(module.id),
                        from_ty: left_ty_id.into_global(module.id),
                        to_ty: right_ty_id.into_global(module.id),
                    });
                }
                // cast returns the target (right) type
                types.get_type(right_ty_id).clone()
            }
            TypeBinaryOperator::Satisfies => {
                // NOTE #Suspicious: unsure about auto unwrapping Type::Value in satisfies?
                // unwrap Type::Value if the right side is a class/struct used as a type
                // (class references have value type = Type::Value { value: instance_ty })
                let target_ty_id = match types.get_type(right_ty_id) {
                    Type::Value { value } => *value,
                    _ => right_ty_id,
                };

                // check if left type satisfies (is assignable to) right type
                if self.check_is_type_assignable(target_ty_id, left_ty_id, types)
                    == Assignability::NotAssignable
                {
                    self.error(AnalyzeError::UnsatisfiedType {
                        node: expression_id.into_global_any(module.id),
                        expected_ty: target_ty_id.into_global(module.id),
                        actual_ty: left_ty_id.into_global(module.id),
                    });
                }
                // satisfies returns the original (left) type, not the asserted type
                types.get_type(left_ty_id).clone()
            }
            _ => {
                // NOTE #Incomplete: other type level binary operations (is, instanceof, extends, etc.)
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                }
            }
        }
    }

    /// Infer the result type of a value of operation.
    pub(super) fn infer_value_of_operation(
        &self,
        _mutability: Option<Mutability>,
        _variance: Option<VarianceBound>,
        right: &Type,
    ) -> Type {
        // NOTE #Incomplete: resolve value of operation type
        right.clone()
    }

    /// Infer the result type of a reference of operation.
    pub(super) fn infer_reference_of_operation(
        &self,
        _mutability: Option<Mutability>,
        _variance: Option<VarianceBound>,
        right: &Type,
    ) -> Type {
        // NOTE #Incomplete: resolve reference of operation type
        right.clone()
    }

    /// Infer the type of a member field on a type by key.
    pub(super) fn infer_member_of_type(
        &self,
        module: &Module,
        receiver_ty: &Type,
        member_key: &StaticKey,
        types: &mut TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<LocalTypeId> {
        match receiver_ty {
            // object type: look up field directly
            Type::Object { fields } => fields.iter().find(|f| &f.key == member_key).map(|f| f.ty),

            // value type: unwrap to the underlying type
            Type::Value { value } => {
                let value_ty = types.get_type(*value).clone();
                self.infer_member_of_type(module, &value_ty, member_key, types, visited)
            }

            // reference to a nominal type: look up in the declaration instance type and extensions
            Type::Reference { symbol, .. } => {
                self.infer_member_of_symbol(module, *symbol, member_key, types, visited)
            }

            // union type: require all elements to have the field, return union of field types
            Type::Union { elements } => {
                let element_ids = elements.clone();
                let mut field_types: Vec<LocalTypeId> = Vec::new();
                for element_id in element_ids {
                    let element_ty = types.get_type(element_id).clone();
                    if let Some(field_ty) =
                        self.infer_member_of_type(module, &element_ty, member_key, types, visited)
                    {
                        field_types.push(field_ty);
                    } else {
                        return None;
                    }
                }
                // if all field types are the same, return that type
                // otherwise, return a union of the field types
                if field_types.is_empty() {
                    None
                } else if field_types.len() == 1 {
                    Some(field_types[0])
                } else {
                    // check if all types are identical
                    let first = field_types[0];
                    if field_types.iter().all(|&t| t == first) {
                        Some(first)
                    } else {
                        Some(types.insert_type(Type::Union {
                            elements: field_types,
                        }))
                    }
                }
            }

            // intersection type: first match wins
            Type::Intersection { elements } => {
                let element_ids = elements.clone();
                for element_id in element_ids {
                    let element_ty = types.get_type(element_id).clone();
                    if let Some(field_ty) =
                        self.infer_member_of_type(module, &element_ty, member_key, types, visited)
                    {
                        return Some(field_ty);
                    }
                }
                None
            }

            _ => None,
        }
    }

    /// Infer member type for a nominal type symbol, traversing lineage and extensions.
    fn infer_member_of_symbol(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        types: &mut TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<LocalTypeId> {
        // cycle detection: if we've already visited this symbol, stop
        // NOTE #Suspicious: should we really just return None for already visited symbol types?
        if visited.contains(&symbol) {
            return None;
        }
        visited.push(symbol);

        // step 1: look up in the type's own instance type
        if let Some(ty_id) = types.get_instance_type_id(symbol) {
            let ty = types.get_type(ty_id).clone();
            if let Some(member_ty) =
                self.infer_member_of_type(module, &ty, member_key, types, visited)
            {
                return Some(member_ty);
            }
        }

        // step 2: traverse lineage (extends, implements, embedded)
        if let Some(lineage) = types.get_lineage_for_symbol(symbol).cloned() {
            // check parent type (extends)
            if let Some(extends) = lineage.extends
                && let Some(member_ty) =
                    self.infer_member_of_symbol(module, extends, member_key, types, visited)
            {
                return Some(member_ty);
            }

            // check implemented interfaces
            for implements in &lineage.implements {
                if let Some(member_ty) =
                    self.infer_member_of_symbol(module, *implements, member_key, types, visited)
                {
                    return Some(member_ty);
                }
            }

            // check embedded types
            for embedded in &lineage.embedded {
                if let Some(member_ty) =
                    self.infer_member_of_symbol(module, *embedded, member_key, types, visited)
                {
                    return Some(member_ty);
                }
            }
        }

        // step 3: check visible extensions
        let extension_ids = types.get_extensions_for_target(symbol)?.clone();
        for extension_id in extension_ids {
            let extension = types.get_extension(extension_id);
            if !self.is_extension_visible(module, extension) {
                continue;
            }
            if let Some(ty_id) = types.get_instance_type_id(extension.symbol) {
                let ty = types.get_type(ty_id);
                if let Type::Object { fields } = ty
                    && let Some(field) = fields.iter().find(|f| &f.key == member_key)
                {
                    return Some(field.ty);
                }
            }
        }

        None
    }

    /// Check if an extension is visible from the given module:
    /// Native: Extension in same module as target type, always visible wherever type is used.
    /// Anonymous: Extension on foreign type, only visible in the file where it is declared.
    /// Named: Extension on foreign type, must be explicitly imported to use.
    pub(super) fn is_extension_visible(&self, module: &Module, extension: &Extension) -> bool {
        match extension.kind {
            ExtensionKind::Inherent => true,
            ExtensionKind::Local => extension.symbol.module_id == module.id,
            ExtensionKind::Nominal => {
                if extension.symbol.module_id == module.id {
                    return true;
                }
                false // TODO #Incomplete: local/named extensions #Extensions
            }
        }
    }

    /// Resolve a remote symbol's value type by ensuring its module is analyzed
    /// and copying the type into the current module's TypeTable.
    pub(super) fn resolve_remote_symbol_value_type(
        &self,
        _module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        let remote_module_id = target_symbol.module_id;

        // ensure the remote module is analyzed (may yield)
        self.require_analyze_module(remote_module_id, profile)?;

        // look up the type in the remote module's TypeTable
        let remote_module = self.program.modules.get(remote_module_id);
        let remote_module = remote_module.read();
        let remote_types = remote_module.dir(profile).types.read();

        // copy the type into our local TypeTable
        if let Some(remote_ty_id) = remote_types.get_value_type_id(target_symbol) {
            let remote_ty = remote_types.get_type(remote_ty_id);
            let local_ty = self.import_type_from_remote(
                expression_id,
                remote_ty,
                &remote_types,
                target_symbol,
                types,
            );
            Ok(local_ty)
        }
        // remote symbol doesn't have a value type, return unknown
        else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            Ok(types.insert_type_from(ty, expression_id))
        }
    }

    /// Import a type from a remote module into the current module's TypeTable.
    /// For structural types (arrays, objects, ..): recursively copy the type structure.
    /// For nominal types (Type::Reference): keep them as references to the original symbol.
    pub(super) fn import_type_from_remote(
        &self,
        expression_id: LocalNodeId<Expression>,
        remote_ty: &Type,
        remote_types: &TypeTable,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        match remote_ty {
            // leaf types: copy directly
            Type::TypeLiteral { value } => types.insert_type_from(
                Type::TypeLiteral {
                    value: value.clone(),
                },
                expression_id,
            ),
            Type::InferVar { .. } => types.insert_type_from(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                expression_id,
            ),
            Type::Error => types.insert_type_from(Type::Error, expression_id),

            // array types
            Type::Array { element: None } => {
                types.insert_type_from(Type::Array { element: None }, expression_id)
            }
            Type::Array {
                element: Some(element_id),
            } => {
                let element_ty = remote_types.get_type(*element_id);
                let local_elem = self.import_type_from_remote(
                    expression_id,
                    element_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(
                    Type::Array {
                        element: Some(local_elem),
                    },
                    expression_id,
                )
            }

            // tuple types
            Type::Tuple { elements } => {
                let local_elements: Vec<_> = elements
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote(
                            expression_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                types.insert_type_from(
                    Type::Tuple {
                        elements: local_elements,
                    },
                    expression_id,
                )
            }

            // object types
            Type::Object { fields } => {
                let local_fields: Vec<_> = fields
                    .iter()
                    .map(|field| {
                        let ty = remote_types.get_type(field.ty);
                        let local_ty = self.import_type_from_remote(
                            expression_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        );
                        TypeField {
                            key: field.key,
                            ty: local_ty,
                            is_optional: field.is_optional,
                            is_readonly: field.is_readonly,
                        }
                    })
                    .collect();
                types.insert_type_from(
                    Type::Object {
                        fields: local_fields,
                    },
                    expression_id,
                )
            }

            // function types
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                dynamic_parameters,
                return_type,
            } => {
                let local_static_params: Vec<_> = static_parameters
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote(
                            expression_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                let local_dynamic_params: Vec<_> = dynamic_parameters
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote(
                            expression_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                let local_return = return_type.map(|id| {
                    let ty = remote_types.get_type(id);
                    self.import_type_from_remote(
                        expression_id,
                        ty,
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                types.insert_type_from(
                    Type::Function {
                        asynchrony: *asynchrony,
                        cardinality: *cardinality,
                        static_parameters: local_static_params,
                        dynamic_parameters: local_dynamic_params,
                        return_type: local_return,
                    },
                    expression_id,
                )
            }

            // union and intersection types
            Type::Union { elements } => {
                let local_elements: Vec<_> = elements
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote(
                            expression_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                types.insert_type_from(
                    Type::Union {
                        elements: local_elements,
                    },
                    expression_id,
                )
            }
            Type::Intersection { elements } => {
                let local_elements: Vec<_> = elements
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote(
                            expression_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                types.insert_type_from(
                    Type::Intersection {
                        elements: local_elements,
                    },
                    expression_id,
                )
            }

            // type modifiers: recurse into inner type
            Type::Value { value } => {
                let inner_ty = remote_types.get_type(*value);
                let local_inner = self.import_type_from_remote(
                    expression_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(Type::Value { value: local_inner }, expression_id)
            }
            Type::Mutable { mutability, right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote(
                    expression_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(
                    Type::Mutable {
                        mutability: *mutability,
                        right: local_inner,
                    },
                    expression_id,
                )
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote(
                    expression_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(
                    Type::ValueOf {
                        mutability: *mutability,
                        variance: *variance,
                        right: local_inner,
                    },
                    expression_id,
                )
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote(
                    expression_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(
                    Type::ReferenceOf {
                        mutability: *mutability,
                        variance: *variance,
                        right: local_inner,
                    },
                    expression_id,
                )
            }
            Type::Unary { operator, right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote(
                    expression_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(
                    Type::Unary {
                        operator: *operator,
                        right: local_inner,
                    },
                    expression_id,
                )
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                let left_ty = remote_types.get_type(*left);
                let right_ty = remote_types.get_type(*right);
                let local_left = self.import_type_from_remote(
                    expression_id,
                    left_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_right = self.import_type_from_remote(
                    expression_id,
                    right_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(
                    Type::Binary {
                        left: local_left,
                        operator: *operator,
                        right: local_right,
                    },
                    expression_id,
                )
            }

            // nominal/reference types: keep as Type::Reference to the original symbol
            Type::Reference {
                symbol,
                static_arguments,
            } => types.insert_type_from(
                Type::Reference {
                    symbol: *symbol,
                    static_arguments: static_arguments.clone(),
                },
                expression_id,
            ),

            // types that can't be meaningfully copied: fall back to reference
            Type::Unevaluated(_) | Type::ArraySized { .. } => types.insert_type_from(
                Type::Reference {
                    symbol: target_symbol,
                    static_arguments: None,
                },
                expression_id,
            ),
        }
    }

    /// Check whether the receiver explicitly implements a language item interface.
    pub(super) fn is_interface_implemented(
        &self,
        ty: &Type,
        interface_item: LanguageItem,
        types: &TypeTable,
    ) -> bool {
        let interface_symbol = self.expect_language_item(interface_item);
        match ty {
            Type::Reference { symbol, .. } => {
                self.is_type_lineage_assignable(*symbol, interface_symbol, types)
            }
            Type::Union { elements } => elements.iter().all(|element_id| {
                let element_ty = types.get_type(*element_id);
                self.is_interface_implemented(element_ty, interface_item, types)
            }),
            _ => false,
        }
    }

    /// Check whether a type is definitely a struct type.
    pub(super) fn is_definitely_struct_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Reference { symbol, .. } => symbol.ty() == SymbolType::Struct,
            Type::TypeLiteral {
                value: TypeLiteral::Composite(DeclarationType::Struct),
            } => true,
            _ => false,
        }
    }

    /// Check whether a type is unresolved for operator resolution.
    pub(super) fn is_unresolved_operator_type(&self, ty: &Type, types: &TypeTable) -> bool {
        match ty {
            Type::InferVar { .. } => true,
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            } => true,
            Type::Union { elements } => elements.iter().any(|element_id| {
                self.is_unresolved_operator_type(types.get_type(*element_id), types)
            }),
            _ => false,
        }
    }

    /// Check whether a type behaves like a numeric type.
    pub(super) fn is_numeric_like_type(&self, ty: &Type, types: &TypeTable) -> bool {
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(primitive),
            } => matches!(
                primitive,
                PrimitiveType::Number
                    | PrimitiveType::Int(_)
                    | PrimitiveType::Float(_)
                    | PrimitiveType::Bigint
            ),
            Type::TypeLiteral {
                value:
                    TypeLiteral::ScalarLiteral(
                        ScalarLiteral::Integer(_)
                        | ScalarLiteral::Float(_)
                        | ScalarLiteral::Bigint(_),
                    ),
            } => true,
            Type::Union { elements } => elements
                .iter()
                .all(|element_id| self.is_numeric_like_type(types.get_type(*element_id), types)),
            _ => false,
        }
    }

    /// Check whether a type is a primitive or scalar literal for builtin operators.
    pub(super) fn is_primitive_literal_type(&self, ty: &Type, types: &TypeTable) -> bool {
        match ty {
            Type::TypeLiteral {
                value:
                    TypeLiteral::Primitive(_)
                    | TypeLiteral::ScalarLiteral(_)
                    | TypeLiteral::Null
                    | TypeLiteral::Undefined,
            } => true,
            Type::Union { elements } => elements.iter().all(|element_id| {
                self.is_primitive_literal_type(types.get_type(*element_id), types)
            }),
            _ => false,
        }
    }

    /// Extract the return type from a function type.
    pub(super) fn function_return_type(
        &self,
        fn_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        match types.get_type(fn_ty_id) {
            Type::Function { return_type, .. } => *return_type,
            _ => None,
        }
    }

    /// Strip nullish types from a type id.
    pub(super) fn strip_nullish_from_union(
        &self,
        ty_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> (Option<LocalTypeId>, bool) {
        let ty = types.get_type(ty_id);

        match ty {
            Type::Union { elements } => {
                let mut filtered = Vec::new();
                let mut has_nullish = false;

                for element_id in elements {
                    let element_ty = types.get_type(*element_id);
                    if self.is_nullish_type(element_ty) {
                        has_nullish = true;
                    } else {
                        filtered.push(*element_id);
                    }
                }

                if !has_nullish {
                    return (Some(ty_id), false);
                }

                let non_nullish_ty_id = match filtered.len() {
                    0 => None,
                    1 => Some(filtered[0]),
                    _ => Some(types.insert_type(Type::Union { elements: filtered })),
                };

                (non_nullish_ty_id, true)
            }
            _ if self.is_nullish_type(ty) => (None, true),
            _ => (Some(ty_id), false),
        }
    }

    /// Build a union type from two type ids.
    pub(super) fn union_type_ids(
        &self,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        if left_ty_id == right_ty_id {
            return left_ty_id;
        }

        let mut elements = Vec::new();
        self.append_union_elements(left_ty_id, &mut elements, types);
        self.append_union_elements(right_ty_id, &mut elements, types);

        if elements.len() == 1 {
            elements[0]
        } else {
            types.insert_type(Type::Union { elements })
        }
    }

    /// Build a union type from a list of type ids.
    pub(super) fn union_type_ids_from_list(
        &self,
        type_ids: Vec<LocalTypeId>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let mut elements = Vec::new();
        for ty_id in type_ids {
            self.append_union_elements(ty_id, &mut elements, types);
        }

        if elements.len() == 1 {
            elements[0]
        } else {
            types.insert_type(Type::Union { elements })
        }
    }

    /// Append union elements for a type id to a list.
    fn append_union_elements(
        &self,
        ty_id: LocalTypeId,
        elements: &mut Vec<LocalTypeId>,
        types: &TypeTable,
    ) {
        match types.get_type(ty_id) {
            Type::Union { elements: union } => {
                for element_id in union {
                    if !elements.contains(element_id) {
                        elements.push(*element_id);
                    }
                }
            }
            _ => {
                if !elements.contains(&ty_id) {
                    elements.push(ty_id);
                }
            }
        }
    }

    /// Check whether a type is null or undefined.
    fn is_nullish_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::TypeLiteral {
                value: TypeLiteral::Null | TypeLiteral::Undefined,
            }
        )
    }
}
