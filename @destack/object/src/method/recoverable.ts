import { Duration, present, type schema } from "@destack/schema";
import { type Row, type Select } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import type { Call } from "./call.ts";
import { defineMethod, method, type Method, type MethodBuilder } from "./method.ts";
import { Step } from "./step.ts";
import { Empty } from "./procedure.ts";

import { keptCall, type RecoverableDefinition } from "../trait/recoverable.ts";
import type { TraitTable } from "../object/table.ts";
/** The method declarations of recoverable objects whose purge keeps the record. */
const keptMethod: MethodBuilder<
    TraitTable<{ readonly recoverable: RecoverableDefinition & { readonly keep: "record" } }>
> = method;

/** Read the recovery a call's served object type keeps, which its host may set. */
function recoveryOf(call: Call): RecoverableDefinition {
    return present(call.object.lifecycle.recoverable, `the recovery of ${call.object.name}`);
}

/** Restore a deleted object, or purge it for good. */
export function restoration(
    kind: "restore" | "purge",
    permission: string,
    options: RecoverableDefinition,
): Method<{
    kind: "restore" | "purge";
    permission: string;
    prepared: schema.Schema;
    mutates: true;
}> {
    const isRestore = kind === "restore";

    return defineMethod<{
        kind: "restore" | "purge";
        permission: string;
        prepared: schema.Schema;
        mutates: true;
    }>({
        kind,
        permission,
        mutates: true,
        isPredicted: isRestore || options.keep !== "record",
        target: true,
        result: isRestore ? "object" : "value",
        ...(isRestore || options.prepared === undefined ? {} : { prepared: options.prepared }),
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: `/{id}/${kind}` },
            input: shapes.target.extend(shapes.replay),
            output: isRestore ? shapes.row : Empty,
        }),
        handler: isRestore ? restore : purge,
        ...(isRestore
            ? {
                  inverse: (step: Step) => {
                      // delete the restored object to its trash again
                      const call = Step.call(step, "delete", Step.target(step));

                      return call === undefined ? undefined : [call];
                  },
              }
            : {
                  async execute(this: Method, call: Call) {
                      // require a purge handler for kept records
                      if (recoveryOf(call).keep === "record" && this.handler === purge) {
                          throw new TypeError(
                              `object ${call.object.name} keeps purged records, so a purge handler of its own destroys their content`,
                          );
                      }

                      return this.handler(call);
                  },
              }),
    });
}

/** Clear a deletion that is still recoverable. */
async function restore(
    call: Call<TraitTable<{ readonly recoverable: RecoverableDefinition }>>,
): Promise<Select<TraitTable<{ readonly recoverable: RecoverableDefinition }>>> {
    // require an unpurged deletion within its window
    const { object } = call;
    const target = call.requireTarget();
    const requested = target.deletionRequestedAt;
    if (requested === null) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is not deleted` });
    } else if (isPurged(target)) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is purged` });
    } else if (call.now - requested >= Duration.milliseconds(recoveryOf(call).within)) {
        throw new ServiceError("CONFLICT", {
            message: `${object.name} is past its recovery window`,
        });
    }

    return call.update({ deletionRequestedAt: null, deletedBy: null });
}

/** Purge an object in the trash: mark a kept record purged, else remove it. */
async function purge(
    call: Call<TraitTable<{ readonly recoverable: RecoverableDefinition }>>,
): Promise<Record<string, never>> {
    // purge only objects in the trash, once
    const { object } = call;
    const target = call.requireTarget();
    if (target.deletionRequestedAt === null) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is not deleted` });
    } else if (isPurged(target)) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is purged` });
    }

    // mark a kept record purged
    const kept = keptCall(call);
    if (kept !== undefined) {
        await kept.update({ purgedAt: call.now });
    }
    // remove the object at the loaded revision
    else {
        await call.remove();
    }

    return {};
}

/** Remove a purged record the type keeps, at the loaded revision. */
export const discard = keptMethod
    .mutation({ permission: null, isSystem: true, output: Empty })
    .handle(async (call) => {
        // require a purged record
        if (!isPurged(call.target)) {
            throw new ServiceError("CONFLICT", { message: `${call.object.name} is not purged` });
        }

        // delete it at the loaded revision
        await call.remove();

        return {};
    });

/** Decide whether a purge already destroyed a kept record's content. */
function isPurged(row: Row): boolean {
    return row["purgedAt"] !== undefined && row["purgedAt"] !== null;
}
