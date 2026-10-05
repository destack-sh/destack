import { Risk } from "@destack/resource";
import { schema } from "@destack/schema";
import { method, type MethodBuilder } from "./method.ts";

import type { ProvisionedTable } from "../trait/provisioned.ts";
/** The method declarations of provisioned resources. */
export const provisionedMethod: MethodBuilder<ProvisionedTable<unknown>> = method;

/** Make an installation the owner of a retained resource it declares again, or release it, cancelling a deletion. */
export const adopt = provisionedMethod
    .mutation({
        permission: null,
        isSystem: true,
        input: schema.object({
            /** The installation owning the resource, absent to release it. */
            owner: schema.string().min(1).nullable(),
        }),
    })
    .handle((call) => call.update({ owner: call.input.owner, deletionRequestedAt: null }));

/** Write the desired states and approval threshold the resource's owner of intent sets, beside its declared specification. */
export const specify = provisionedMethod
    .mutation({
        permission: null,
        isSystem: true,
        input: schema.object({
            /** The desired states the active consumers need. */
            states: schema.array(schema.record(schema.string(), schema.json())),
            /** The desired states only draining consumers still need. */
            drainingStates: schema.array(schema.record(schema.string(), schema.json())),
            /** The least risk of a plan that waits for approval. */
            approval: Risk,
        }),
    })
    .handle((call) => call.update(call.input));
