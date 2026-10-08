import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, createSignal } from "solid-js";
import { TRANSPARENT } from "./utils.ts";

/** Follow a permission's state, unknown until the browser answers or where it does not report it. */
export function createPermission(
    name: PermissionDescriptor | PermissionName,
): Accessor<PermissionState | "unknown"> {
    // know nothing on the server
    if (isServer) {
        return () => "unknown";
    }

    // query the permission, leaving one the browser cannot query unknown and reporting other failures
    const [permission, setPermission] = createSignal<PermissionState | "unknown">("unknown", {
        ownedWrite: true,
    });
    const [status, setStatus] = createSignal<PermissionStatus | undefined>(undefined, {
        ownedWrite: true,
    });
    const descriptor: PermissionDescriptor = typeof name === "string" ? { name } : name;
    navigator.permissions.query(descriptor).then(
        (answered) => setStatus(() => answered),
        (error: unknown) => {
            if (!(error instanceof TypeError)) {
                reportError(error);
            }
        },
    );

    // follow the state of the answered status
    createEffect(
        status,
        (current) => {
            if (current === undefined) {
                return undefined;
            }
            const update = (): void => {
                setPermission(current.state);
            };
            update();
            current.addEventListener("change", update);

            return () => current.removeEventListener("change", update);
        },
        TRANSPARENT,
    );

    return permission;
}
