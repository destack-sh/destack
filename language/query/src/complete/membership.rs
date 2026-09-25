use tspp_core::FxIndexMap;
use tspp_dir as dir;

use crate::format::Reopening;
use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult};

/// One member a projected membership exposes at a site.
pub(crate) struct ExposedMember {
    /// The member binding, declared-form for source members.
    pub(crate) binding: dir::MemberBinding,
    /// The reopening declared types format under, absent for structural members.
    pub(crate) reopening: Option<Reopening>,
}

/// Collect the members one site's projected membership exposes.
pub(crate) fn membership_members(
    module: &ModuleQueryContext<'_>,
    program: &ProgramQueryContext<'_>,
    site: dir::MemberSite,
) -> QueryResult<Vec<ExposedMember>> {
    // read the membership the site's subject selects
    let members = module.members()?;
    let (subject, _) = members
        .subject(site)
        .ok_or(QueryError::missing(format!("member subject: {site:?}")))?;
    let membership = members
        .membership(site)
        .ok_or(QueryError::missing(format!(
            "member membership: site={site:?}, subject={subject:?}"
        )))?
        .clone();

    // expose each source owner's declared bindings under its reopening
    let mut results = Vec::new();
    for source in &membership.sources {
        let owner = program.module(source.owner.module_id)?;
        let owner_members = owner.members()?;
        let Some(bindings) = owner_members.bindings(source.owner, subject.space) else {
            continue;
        };
        let bindings = bindings.to_vec();
        let reopening = source_reopening(module, program, source, membership.receiver)?;

        // keep the first source declaring each key
        for binding in bindings {
            if results
                .iter()
                .any(|member: &ExposedMember| member.binding.key == binding.key)
            {
                continue;
            }

            results.push(ExposedMember {
                binding,
                reopening: Some(reopening.clone()),
            });
        }
    }

    // expose the structural bindings the projection settled
    for binding in &membership.structural {
        if results
            .iter()
            .any(|member: &ExposedMember| member.binding.key == binding.key)
        {
            continue;
        }

        results.push(ExposedMember {
            binding: binding.clone(),
            reopening: None,
        });
    }

    Ok(results)
}

/// Find the member one site's projected membership exposes at one key.
pub(crate) fn membership_member(
    module: &ModuleQueryContext<'_>,
    program: &ProgramQueryContext<'_>,
    site: dir::MemberSite,
    key: dir::StaticKey,
) -> QueryResult<Option<ExposedMember>> {
    let members = membership_members(module, program, site)?;

    Ok(members.into_iter().find(|member| member.binding.key == key))
}

/// Build the reopening one source's declared bindings format under.
fn source_reopening(
    module: &ModuleQueryContext<'_>,
    program: &ProgramQueryContext<'_>,
    source: &dir::MemberSource,
    receiver: dir::GlobalTypeId,
) -> QueryResult<Reopening> {
    // resolve the stored arguments in the membership's own module
    let arguments = module.types()?.type_ids(source.arguments).to_vec();

    // pair the owner's declared parameters with the arguments in order
    let owner = program.module(source.owner.module_id)?;
    let mut parameters = FxIndexMap::default();
    if let Some(template) = owner.generics()?.template_by_symbol(source.owner) {
        let declared = owner.generics()?.get_template(template).parameters.clone();
        for (parameter, argument) in declared.iter().zip(arguments) {
            let parameter = parameter.into_global(source.owner.module_id);

            // skip an identity binding, where the owner's parameter reopens into itself
            let bound = program
                .module(argument.module_id)?
                .types()?
                .get_type(argument.local_id);
            if matches!(bound, dir::Type::Parameter(bound) if bound == parameter) {
                continue;
            }

            parameters.insert(parameter, argument);
        }
    }

    Ok(Reopening {
        parameters,
        receiver: Some(receiver),
    })
}
