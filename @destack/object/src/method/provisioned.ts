import { Risk } from "@destack/resource";
import { Digest, schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";

import { ObjectError } from "../error/error.ts";
import { Execution } from "./execution.ts";
import { MethodBuilder } from "./record.ts";
import type { ResourceTable } from "../trait/provisioned.ts";
/** The method declarations of provisioned resources. */
export const provisionedMethod = new MethodBuilder<ResourceTable<unknown>>();

/** Make an installation the owner of a retained resource it declares again, or release it. */
export const adopt = provisionedMethod
    .mutation({
        permission: "declare",
        visibility: "installation",
        input: schema.object({
            /** The owner installation of the resource, absent to release it. */
            owner: ObjectReference.nullable(),
        }),
    })
    .handle((context) =>
        context.update({
            ...ObjectReference.columns("owner", context.input.owner ?? undefined),
            deletedAt: null,
        }),
    );

/** Write the desired states' digests and the approval threshold. */
export const specify = Execution.handle(
    provisionedMethod.mutation({
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
        const missing = await blobs.storage.missing([...new Set([...states, ...drainingStates])]);
        if (missing.length > 0) {
            throw ObjectError.of(
                "DESIRED_STATE_MISSING",
                `${call.scope} has no desired states ${missing.join(", ")}`,
            );
        }

        return await call.update(call.input);
    },
);
