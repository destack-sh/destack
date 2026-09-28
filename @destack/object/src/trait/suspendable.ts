import type { Method } from "../method/method.ts";
import { custom } from "./record.ts";
import type { Gated, Trait } from "./trait.ts";

/** The methods suspendable scope objects take. */
export type SuspendableMethodMap<Suspend> = [Suspend] extends [string]
    ? {
          readonly suspend: Method<"custom", Suspend, never, never, true>;
          readonly resume: Method<"custom", Suspend, never, never, true>;
      }
    : {};

/** Scope objects that can be suspended. */
export const suspendable: Trait<Gated> & {
    /** Declare the method suspending a scope object. */
    suspend<const Permission extends string>(
        permission: Permission,
    ): Method<"custom", Permission, never, never, true>;
    /** Declare the method resuming a suspended scope object. */
    resume<const Permission extends string>(
        permission: Permission,
    ): Method<"custom", Permission, never, never, true>;
} = {
    key: "suspendable",
    isDurable: true,
    options: (definition) => definition.suspendable,
    columns: () => ({}),
    constraints: () => [],
    methods: (options) => ({
        suspend: suspendable.suspend(options.by),
        resume: suspendable.resume(options.by),
    }),
    validate: (_options, object, definition) => {
        // require a scope type
        if (definition.isScope !== true) {
            throw new TypeError(`object ${object.name} is suspendable but no scope`);
        }
    },
    suspend: (permission) => change(permission, "suspend", "resume"),
    resume: (permission) => change(permission, "resume", "suspend"),
};

/** Suspend or resume a scope object on the server. */
function change<const Permission extends string>(
    permission: Permission,
    change: "suspend" | "resume",
    inverse: "suspend" | "resume",
): Method<"custom", Permission, never, never, true> {
    const handled = custom({ permission, inverse }).handle(async (call) => {
        await call.served()[change](call.reference());

        return call.target;
    });

    return { ...handled, isPredicted: false };
}
