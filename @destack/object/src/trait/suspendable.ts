import { method, type Method } from "../method/method.ts";
import type { Gated, Trait } from "./trait.ts";

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
    suspend<const Permission extends string>(
        permission: Permission,
    ): Method<{ kind: "custom"; permission: Permission; mutates: true }>;
    /** Declare the method resuming a suspended scope object. */
    resume<const Permission extends string>(
        permission: Permission,
    ): Method<{ kind: "custom"; permission: Permission; mutates: true }>;
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
    action: "suspend" | "resume",
    inverse: "suspend" | "resume",
): Method<{ kind: "custom"; permission: Permission; mutates: true }> {
    const handled = method.mutation({ permission, inverse }).handle(async (call) => {
        await call.requireAuthorization()[action](call.reference());

        return call.target;
    });

    return { ...handled, isPredicted: false };
}
