import { afterEach, expect, test } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createPermission } from "./permission.ts";

/** A stand-in permission status whose state the test changes. */
class StubStatus extends EventTarget implements PermissionStatus {
    /** The permission's name. */
    readonly name: string;
    /** The permission's state. */
    state: PermissionState;
    /** The change handler, unused. */
    onchange = null;

    /** Hold the name and the first state. */
    constructor(name: string, state: PermissionState) {
        super();
        this.name = name;
        this.state = state;
    }

    /** Change the state, telling the listeners. */
    change(state: PermissionState): void {
        this.state = state;
        this.dispatchEvent(new Event("change"));
    }
}

afterEach(() => {
    Reflect.deleteProperty(navigator, "permissions");
});

/** Answer permission queries with statuses, refusing names without one. */
function stubPermissions(statuses: Record<string, StubStatus>): void {
    Object.defineProperty(navigator, "permissions", {
        configurable: true,
        value: {
            query: async ({ name }: PermissionDescriptor) => {
                const status = statuses[name];
                if (status === undefined) {
                    throw new TypeError(`${name} is not a permission name`);
                }

                return status;
            },
        },
    });
}

/** Wait for the queries to answer. */
async function settle(): Promise<void> {
    await new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
    flush();
}

test("follow a permission's state from unknown through its answer and changes", async () => {
    // query the camera and an unqueryable permission
    const camera = new StubStatus("camera", "denied");
    stubPermissions({ camera });
    const { states, dispose } = createRoot((disposeRoot) => ({
        states: [createPermission("camera"), createPermission({ name: "midi" })],
        dispose: disposeRoot,
    }));
    const before = states.map((state) => state());
    await settle();
    const answered = states.map((state) => state());

    // grant the camera
    camera.change("granted");
    flush();
    const changed = states.map((state) => state());
    dispose();

    expect([before, answered, changed]).toEqual([
        ["unknown", "unknown"],
        ["denied", "unknown"],
        ["granted", "unknown"],
    ]);
});
