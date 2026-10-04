import { none } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { RelationalQuery, type Submission } from "@destack/object/client";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import type { ScopeCommands } from "../scope/scope.ts";

/** A note the model-bound components show and change. */
export const note = defineObject({
    name: "note",
    plural: "notes",
    scope: Scope.universe.id,
    fields: {
        title: field.string(schema.string().min(1).max(20)),
        words: field.integer(),
        pinned: field.boolean(),
        kind: field.enum(["idea", "task"]),
        status: field.state({
            initial: "draft",
            transitions: {
                publish: { from: ["draft"], to: "published", permission: "write" },
                archive: { from: ["draft", "published"], to: "archived", permission: "manage" },
                restore: { from: ["archived"], to: "draft", permission: "write" },
            },
        }),
    },
    permissions: { read: none(), write: none(), manage: none() },
    methods: (method) => ({ update: method.update("write") }),
});

/** A submission whose confirmation the test settles. */
export interface Settled<Value> {
    /** The submission a write returns. */
    readonly submission: Submission<Value>;
    /** Confirm the write. */
    readonly confirm: () => void;
    /** Refuse the write. */
    readonly refuse: () => void;
}

/** Make a submission predicted at once, confirmed or refused when the test says. */
export function settled<Value>(value: Value): Settled<Value> {
    const { promise, resolve, reject } = Promise.withResolvers<void>();
    promise.catch(() => undefined);

    return {
        submission: { predicted: Promise.resolve(value), confirmed: promise },
        confirm: () => resolve(),
        refuse: () => reject(new Error("refused")),
    };
}

/** Make a live query of fixed rows that the query's search narrows, as a client's copy answers it. */
export function queryOf<Value>(read: () => Value): RelationalQuery<Value> {
    return new RelationalQuery(note, {}, () => ({
        ready: Promise.resolve(),
        read: () => Promise.resolve(read()),
        async *watch(signal: AbortSignal) {
            yield read();
            await new Promise((resolve) => {
                signal.addEventListener("abort", resolve);
            });
        },
        close: () => Promise.resolve(),
    }));
}

/** Make a scope that grants the listed permissions and records each method it calls by name. */
export function commandsOf(granted: readonly string[], calls: string[]): ScopeCommands {
    return {
        can: (permission) => Promise.resolve(granted.includes(permission)),
        call: (_object, method) => {
            calls.push(method);

            return settled<unknown>(method).submission;
        },
    };
}
