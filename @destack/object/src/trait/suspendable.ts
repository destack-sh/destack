import { integer } from "@destack/db";
import { type Method } from "../method/method.ts";
import { method } from "../method/builder.ts";
import type { TraitPermission, TraitBehavior } from "./trait.ts";
import { callableTrait } from "./trait.ts";
import { Execution } from "../method/execution.ts";
import { t } from "@destack/locale";

/** The methods suspendable scope objects take. */
export type SuspendableMethodMap<Suspend> = [Suspend] extends [string]
    ? {
          readonly suspend: Method<{ kind: "custom"; permission: Suspend; mutates: true }>;
          readonly resume: Method<{ kind: "custom"; permission: Suspend; mutates: true }>;
      }
    : {};

/** Scope objects that can be suspended. */
const suspendableTrait: TraitBehavior<TraitPermission> = {
    key: "suspendable",
    storages: ["durable"],
    options: (definition) => definition.suspendable,
    columns: () => Suspendable.columns(),
    constraints: () => [],
    methods: (options) => ({
        suspend: Suspendable.method("suspend", options.by),
        resume: Suspendable.method("resume", options.by),
    }),
    validate: (_options, object, definition) => {
        // require a scope type
        if (definition.isScope !== true) {
            throw new TypeError(`object ${object.name} is suspendable but not a scope`);
        }
    },
};

/** Let scope objects be suspended. */
export const suspendable = callableTrait("suspendable", suspendableTrait);

/** The column, methods and changes between a running scope object and a suspended one. */
export const Suspendable = {
    /** Build the column recording when a scope object was suspended. */
    columns: suspensionColumns,

    /** Declare the method suspending or resuming a scope object, undone by the other. */
    method<const Permission extends string>(
        kind: "suspend" | "resume",
        permission: Permission,
    ): Method<{ kind: "custom"; permission: Permission; mutates: true }> {
        const inverse = kind === "suspend" ? "resume" : "suspend";

        return Execution.handle(
            method.mutation({
                title: t.context("action")`Change`,
                permission,
                inverse,
                isPredicted: false,
            }),
            async (call, execution) => {
                // record it on the row, which the scope's home follows, and on the scope where it is home here
                const store = Execution.store(execution, call);
                const target = await call.update({
                    suspendedAt: kind === "suspend" ? call.now : null,
                });
                const reference = call.reference();
                if (await store.store.authorizer.isHome(store.transaction.view(), reference.id)) {
                    await store.authorization[kind](reference);
                }

                return target;
            },
        );
    },
};

/** Build the column recording when a scope object was suspended. */
function suspensionColumns() {
    return {
        /** The time the scope was suspended, null while it runs. */
        suspendedAt: integer("suspended_at"),
    };
}

/** The column of a suspendable scope object. */
export type SuspensionColumns = ReturnType<typeof suspensionColumns>;
