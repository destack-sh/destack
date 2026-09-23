import { implement, type ServiceContext } from "@destack/service/server";
import { ServiceError } from "@destack/service/error";
import { settingService } from "../service/index.ts";
import {
    getPolicy,
    listPolicy,
    mutatePolicy,
    type PolicyEdit,
    type SettingStore,
} from "../database/index.ts";
import type { SettingServerOptions } from "./server.ts";
import type { SettingPolicy } from "../setting/policy.ts";
import { reportSettingError } from "./error.ts";

/** Connect administrative policy operations to the shared transactional store. */
export function policyRouter(store: SettingStore, options: SettingServerOptions) {
    const implementation = implement(settingService.policy).$context<ServiceContext>();

    return {
        get: implementation.get.handler(async ({ input, context }) => {
            await options.authorizePolicy(context, input.authority, "read");
            const policy = await getPolicy(store.database, input.authority, input.id);
            if (!policy) {
                throw new ServiceError("NOT_FOUND");
            }

            return policy;
        }),
        list: implementation.list.handler(async ({ input, context }) => {
            await options.authorizePolicy(context, input.authority, "read");

            return listPolicy(store.database, input.authority, input);
        }),
        set: implementation.set.handler(async ({ input, context }) => {
            await options.authorizePolicy(context, input.authority, "write");
            const declarations = await options.declarations(
                context,
                input.packageId,
                input.installationId,
            );
            const setting = declarations.find(
                (candidate) =>
                    candidate.reference.packageId === input.setting.packageId &&
                    candidate.reference.name === input.setting.name,
            );
            if (!setting) {
                throw new ServiceError("NOT_FOUND");
            }
            if (!setting.declaration.schema.safeParse(input.value).success) {
                throw new ServiceError("BAD_REQUEST", {
                    message: "policy value does not match its setting declaration",
                });
            }
            const {
                authority,
                id,
                requestId,
                expectedRevision,
                packageId: _packageId,
                ...definition
            } = input;

            return change(store, options, context, {
                authority,
                id,
                requestId,
                expectedRevision,
                operation: "set",
                definition: { authority, ...definition },
            });
        }),
        remove: implementation.remove.handler(({ input, context }) =>
            change(store, options, context, {
                ...input,
                expectedRevision: input.revision,
                operation: "remove",
            }),
        ),
        detach: implementation.detach.handler(({ input, context }) =>
            change(store, options, context, {
                ...input,
                expectedRevision: input.revision,
                operation: "detach",
            }),
        ),
    };
}

/** Recheck authority before claiming any idempotent mutation. */
function change(
    store: SettingStore,
    options: SettingServerOptions,
    context: ServiceContext,
    input: PolicyEdit & { operation: "remove" },
): Promise<null>;
/** Set or detach a policy after checking administrative authority. */
function change(
    store: SettingStore,
    options: SettingServerOptions,
    context: ServiceContext,
    input: PolicyEdit & { operation: "set" | "detach" },
): Promise<SettingPolicy>;
/** Apply an authorized policy transition. */
async function change(
    store: SettingStore,
    options: SettingServerOptions,
    context: ServiceContext,
    input: PolicyEdit,
) {
    await options.authorizePolicy(context, input.authority, "write");

    return mutatePolicy(store, input, {
        subject: context.requireCaller().authentication.subject,
        audit: await options.audit(context),
    }).catch(reportSettingError);
}
