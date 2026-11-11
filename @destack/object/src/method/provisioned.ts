import { Risk } from "@destack/resource";
import { Digest, schema } from "@destack/schema";

import { ObjectError } from "../error/error.ts";
import { Execution } from "./execution.ts";
import { MethodBuilder } from "./record.ts";
import type { ResourceTable } from "../trait/provisioned.ts";
import { t } from "@destack/locale";
/** The method declarations of provisioned resources. */
export const provisionedMethod = new MethodBuilder<ResourceTable<unknown>>();

/** Cancel a retiring resource's deletion, as its declaration declares it again. */
export const restore = provisionedMethod
    .mutation({ title: t`Restore`, permission: "declare", visibility: "installation" })
    .handle((context) => context.update({ deletedAt: null }));

/** Write the desired states' digests and the approval threshold. */
export const specify = Execution.handle(
    provisionedMethod.mutation({
        title: t`Specify`,
        permission: "declare",
        visibility: "installation",
        input: schema.object({
            /** The digests of the states the active consumers need. */
            states: schema.array(Digest),
            /** The digests of the states only draining consumers still need. */
            drainingStates: schema.array(Digest),
            /** The least risk of a plan that waits for approval. */
            approval: Risk,
        }),
    }),
    async (call, execution) => {
        // refuse a state without a blob, then write the digests
        const { states, drainingStates } = call.input;
        const { blobs } = Execution.store(execution, call).store;
        const missing = await blobs.missing([...new Set([...states, ...drainingStates])]);
        if (missing.length > 0) {
            throw ObjectError.of(
                "STATE_REQUIRED",
                `desired state set '${missing.join(", ")}' of '${call.scope}' is required`,
            );
        }

        return await call.update(call.input);
    },
);
