import { Risk } from "@destack/resource";
import { Digest, schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { DesiredState } from "../trait/desired.ts";

import { MethodBuilder } from "./record.ts";
import type { ResourceTable } from "../trait/provisioned.ts";
import { Execution } from "./execution.ts";
/** The method declarations of provisioned resources. */
export const provisionedMethod = new MethodBuilder<ResourceTable<unknown>>();

/** Make an installation the owner of a retained resource it declares again, or release it. */
export const adopt = provisionedMethod
    .mutation({
        permission: "declare",
        isInternal: true,
        input: schema.object({
            /** The installation owning the resource, absent to release it. */
            owner: ObjectReference.nullable(),
        }),
    })
    .handle((context) =>
        context.update({
            ...ObjectReference.columns("owner", context.input.owner ?? undefined),
            deletedAt: null,
        }),
    );

/** Write the digests of the desired states and the approval threshold of the owner of intent. */
export const specify = Execution.handle(
    provisionedMethod.mutation({
        permission: "declare",
        isInternal: true,
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
        // require a kept state of each digest
        const { states, drainingStates } = call.input;
        const needed = [...new Set([...states, ...drainingStates])];
        await DesiredState.read(Execution.snapshot(execution), call.scope, needed);

        // write the digests and drop the states no resource of the scope needs any more
        const target = call.target;
        const updated = await call.update(call.input);
        const left = [...target.states, ...target.drainingStates].filter(
            (digest) => !needed.includes(digest),
        );
        if (left.length > 0) {
            await DesiredState.drop(call, execution, left);
        }

        return updated;
    },
);
