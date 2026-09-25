use tspp_dir as dir;

use crate::sema::{
    Cause, CauseKind, CheckState, Origin, ProtocolCall, Relation, RelationCheck, Value, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select the disposal protocol calls one using binding runs, when its resource disposes.
    pub(in crate::sema) fn select_disposal(
        &mut self,
        anchor: dir::GlobalNodeIdAny,
        pattern: dir::LocalNodeId<dir::Pattern>,
        asynchrony: dir::Asynchrony,
    ) -> CompilerResult<Option<dir::DisposalDecision>> {
        let module = anchor.module_id;

        // require an async body for an await using
        let is_async_body = self
            .flow
            .current_function()
            .is_some_and(|function| function.asynchrony == dir::Asynchrony::Async);
        if asynchrony == dir::Asynchrony::Async && !is_async_body {
            self.report_await_outside_async_context(module, anchor.local_id);
        }

        // read the bound resource's type, nullish arms disposing nothing
        let Some(symbol) = self.module(module).declaration_symbol(pattern.into_any()) else {
            return Ok(None);
        };
        let site = self.visit_site(anchor)?;
        let origin = site.origin();
        let ty = self.symbol_type(symbol)?;
        let resource = match self.split_nullish_type(origin, ty)? {
            Some(split) => split.value,
            None => ty,
        };
        if matches!(
            self.ty(resource)?,
            dir::Type::Null | dir::Type::Undefined | dir::Type::Never | dir::Type::Error
        ) {
            return Ok(None);
        }

        // select the asynchronous protocol first, falling back to the synchronous one
        let protocols: &[(dir::LanguageItem, &str)] = match asynchrony {
            dir::Asynchrony::Sync => &[(dir::LanguageItem::Dispose, "dispose")],
            dir::Asynchrony::Async => &[
                (dir::LanguageItem::AsyncDispose, "asyncDispose"),
                (dir::LanguageItem::Dispose, "dispose"),
            ],
        };
        let value = Value {
            ty: resource,
            node: None,
            place: None,
            is_fresh: false,
        };
        let mut selected = None;
        for (item, key) in protocols {
            let key = dir::StaticKey::Name(self.strings().intern(key));

            // probe the protocol as a decision, discarding the bound a failure would leave
            let (probe, verdict) = self.decide(|check| {
                check.select_language_protocol_call(
                    origin,
                    value,
                    resource,
                    dir::MemberSpace::Instance,
                    key,
                    *item,
                    &[],
                    &[],
                    &[],
                )
            })?;
            if verdict == Verdict::Fails || probe.is_none() {
                continue;
            }

            // select the protocol the resource implements for real
            let call = self.select_language_protocol_call(
                origin,
                value,
                resource,
                dir::MemberSpace::Instance,
                key,
                *item,
                &[],
                &[],
                &[],
            )?;
            if let Some((_, call)) = call {
                selected = Some((*item, call));
                break;
            }
        }

        // report a resource outside both protocols
        let Some((item, dispose)) = selected else {
            self.report_using_resource_not_disposable(pattern.into_global_any(module));

            return Ok(None);
        };

        // park the completion of an asynchronous disposal
        let awaits = match item {
            dir::LanguageItem::AsyncDispose => {
                self.select_await_park(origin, dispose.return_type)?
            }
            _ => None,
        };

        // record the protocol calls the binding lowers through
        let dir::OperationResolution::One(dispose) = dispose.resolution else {
            return Err(CompilerError::Internal {
                message: "a disposal protocol selected on a union receiver".to_owned(),
            });
        };
        let awaits = awaits.and_then(|park| match park.resolution {
            dir::OperationResolution::One(call) => Some(call),
            _ => None,
        });

        Ok(Some(dir::DisposalDecision { dispose, awaits }))
    }

    /// Select the park awaiting one type an implicit await produces, when it is awaitable.
    pub(in crate::sema) fn select_await_park(
        &mut self,
        origin: Origin,
        awaited: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ProtocolCall>> {
        let parked = self.reduce_operation_type(
            origin,
            dir::TypeOperation::Awaited(dir::UnaryType { target: awaited }),
        )?;
        let value = Value {
            ty: awaited,
            node: None,
            place: None,
            is_fresh: false,
        };
        let key = dir::StaticKey::Name(self.strings().intern("park"));
        let selected = self.select_language_protocol_call(
            origin,
            value,
            awaited,
            dir::MemberSpace::Static,
            key,
            dir::LanguageItem::Awaitable,
            &[parked],
            &[parked],
            &[dir::ArgumentSource::Supplied(0)],
        )?;
        let Some((_, park)) = selected else {
            return Ok(None);
        };

        // bind the park's parameter to the value it receives
        if let dir::OperationResolution::One(call) = &park.resolution
            && let Some(binding) = call.arguments.first()
        {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.push_relation(RelationCheck::new(
                origin,
                Relation::Subtype,
                awaited,
                binding.parameter_type,
                cause,
            ))?;
        }

        Ok(Some(park))
    }
}
