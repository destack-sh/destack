use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, DecoratorCall, StaticIfDecorator};

/// Decorator classified for check-time consumers.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Decorator {
    /// Static inclusion guard.
    StaticIf(StaticIfDecorator),
    /// Compiler language item marker.
    LanguageItem(DecoratorCall),
    /// Compiler intrinsic marker.
    Intrinsic(DecoratorCall),
    /// Representation decorator.
    Representation(DecoratorCall),
    /// Capture policy decorator.
    Capture(DecoratorCall),
    /// Diagnostic policy decorator.
    Diagnostic(DecoratorCall),
    /// Foreign linkage decorator.
    Foreign(DecoratorCall),
    /// Compile-time restriction decorator.
    Restriction(DecoratorCall),
    /// System or codegen hint decorator.
    System(DecoratorCall),
    /// Stability annotation decorator.
    Stability(DecoratorCall),
    /// Taint or safety annotation decorator.
    Taint(DecoratorCall),
    /// Macro provider decorator.
    Macro(DecoratorCall),
    /// Ordinary annotation or macro candidate.
    Other(DecoratorCall),
}

impl CheckState<'_> {
    /// Return classified decorators attached to one owner node.
    pub(in crate::check) fn decorators_for_owner(
        &self,
        module: ModuleId,
        owner: dir::LocalNodeIdAny,
    ) -> Vec<Decorator> {
        self.decorator_calls_for_owner(module, owner)
            .into_iter()
            .map(|call| self.decorator_from_call(module, call))
            .collect()
    }

    /// Classify one decorator call.
    fn decorator_from_call(&self, module: ModuleId, call: DecoratorCall) -> Decorator {
        if let Some(decorator) = self.static_if_decorator_from_call(module, &call) {
            return Decorator::StaticIf(decorator);
        }

        match self.decorator_language_item(module, &call) {
            Some(dir::LanguageItem::LanguageItem) => Decorator::LanguageItem(call),
            Some(dir::LanguageItem::Intrinsic) => Decorator::Intrinsic(call),
            Some(
                dir::LanguageItem::ReprDecorator
                | dir::LanguageItem::AlignDecorator
                | dir::LanguageItem::PackedDecorator,
            ) => Decorator::Representation(call),
            Some(dir::LanguageItem::Capture) => Decorator::Capture(call),
            Some(
                dir::LanguageItem::Allow
                | dir::LanguageItem::Warn
                | dir::LanguageItem::Deny
                | dir::LanguageItem::Forbid
                | dir::LanguageItem::Expect,
            ) => Decorator::Diagnostic(call),
            Some(dir::LanguageItem::Extern) => Decorator::Foreign(call),
            Some(
                dir::LanguageItem::NoManaged
                | dir::LanguageItem::NoHeap
                | dir::LanguageItem::NoRuntime
                | dir::LanguageItem::NoUnsafe
                | dir::LanguageItem::NoDynamicDispatch
                | dir::LanguageItem::NoReflection
                | dir::LanguageItem::NoUnwind
                | dir::LanguageItem::ExclusiveMutableBorrows,
            ) => Decorator::Restriction(call),
            Some(
                dir::LanguageItem::Inline
                | dir::LanguageItem::Noinline
                | dir::LanguageItem::Unroll
                | dir::LanguageItem::Hot
                | dir::LanguageItem::Cold
                | dir::LanguageItem::Likely
                | dir::LanguageItem::Unlikely
                | dir::LanguageItem::MustUse
                | dir::LanguageItem::Pure
                | dir::LanguageItem::Tailcall,
            ) => Decorator::System(call),
            Some(dir::LanguageItem::Deprecated | dir::LanguageItem::Experimental) => {
                Decorator::Stability(call)
            }
            Some(
                dir::LanguageItem::Taint
                | dir::LanguageItem::Untaint
                | dir::LanguageItem::Source
                | dir::LanguageItem::Sink
                | dir::LanguageItem::Unsafe
                | dir::LanguageItem::Safe,
            ) => Decorator::Taint(call),
            Some(
                dir::LanguageItem::Tagged
                | dir::LanguageItem::CloneDerive
                | dir::LanguageItem::DebugDerive,
            ) => Decorator::Macro(call),
            _ => Decorator::Other(call),
        }
    }

    /// Return the language item referenced by one ordinary decorator call.
    fn decorator_language_item(
        &self,
        module: ModuleId,
        call: &DecoratorCall,
    ) -> Option<dir::LanguageItem> {
        let path = call.path.as_ref()?;
        let [name] = path.segments.as_slice() else {
            return None;
        };

        // prefer lexical bindings when they resolve
        if let Some(symbol) = self
            .lookup_symbol_by_name(
                module,
                call.callee.into_any(),
                *name,
                dir::SymbolSpace::Value,
            )
            .unique_symbol()
        {
            return self.environment.language.item(symbol);
        }

        // fall back to compiler known decorator exports
        let name = self.module(module).strings.get(*name);
        let symbol = self.environment.language.symbol_by_name(name)?;

        self.environment.language.item(symbol)
    }
}
