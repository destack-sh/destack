import { method, type Method } from "./method.ts";

/** Suspend or resume a scope object on the server. */
export function change<const Permission extends string>(
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
