import { integer } from "@destack/db";
import { type Method } from "../method/method.ts";
import { method } from "../method/record.ts";
import type { Gated, Trait } from "./trait.ts";
import { Execution } from "../method/execution.ts";

/** The methods suspendable scope objects take. */
export type SuspendableMethodMap<Suspend> = [Suspend] extends [string]
    ? {
          readonly suspend: Method<{ kind: "custom"; permission: Suspend; mutates: true }>;
          readonly resume: Method<{ kind: "custom"; permission: Suspend; mutates: true }>;
      }
    : {};

/** Scope objects that can be suspended. */
export const suspendable: Trait<Gated> & {
    /** Declare the method suspending a scope object. */
    readonly suspend: <const Permission extends string>(
        permission: Permission,
    ) => Method<{ kind: "custom"; permission: Permission; mutates: true }>;
    /** Declare the method resuming a suspended scope object. */
    readonly resume: <const Permission extends string>(
        permission: Permission,
    ) => Method<{ kind: "custom"; permission: Permission; mutates: true }>;
    /** Declare the method suspending or resuming a scope object on the server. */
    readonly change: <const Permission extends string>(
        permission: Permission,
        action: "suspend" | "resume",
        inverse: "suspend" | "resume",
    ) => Method<{ kind: "custom"; permission: Permission; mutates: true }>;
} = {
    key: "suspendable",
    storages: ["durable"],
    options: (definition) => definition.suspendable,
    columns: () => suspensionColumns(),
    constraints: () => [],
    methods: (options) => ({
        suspend: suspendable.suspend(options.by),
        resume: suspendable.resume(options.by),
    }),
    validate: (_options, object, definition) => {
        // require a scope type
        if (definition.isScope !== true) {
            throw new TypeError(`object ${object.name} is suspendable but not a scope`);
        }
    },
    suspend: (permission) => suspendable.change(permission, "suspend", "resume"),
    resume: (permission) => suspendable.change(permission, "resume", "suspend"),
    change(permission, action, inverse) {
        return Execution.handle(
            method.mutation({ permission, inverse, isPredicted: false }),
            async (call, execution) => {
                // record it on the row, which the scope's home follows, and on the scope where it is home here
                const store = Execution.store(execution, call);
                const target = await call.update({
                    suspendedAt: action === "suspend" ? call.now : null,
                });
                const reference = call.reference();
                if (await store.store.authorizer.isHome(store.transaction.view(), reference.id)) {
                    await store.authorization[action](reference);
                }

                return target;
            },
        );
    },
};

/** Build the column recording when a scope object was suspended. */
export function suspensionColumns() {
    return {
        /** The time the scope was suspended, null while it runs. */
        suspendedAt: integer("suspended_at"),
    };
}
