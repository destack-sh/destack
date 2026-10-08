import { isServer } from "@solidjs/web";
import { type Accessor, createSignal } from "solid-js";
import { makeEventListener } from "./event-listener.ts";

/** The screen's orientation: its angle in degrees and its type. */
export interface OrientationState {
    /** The angle in degrees: 0, 90, 180 or 270. */
    readonly angle: number;
    /** The orientation's type, such as `portrait-primary`. */
    readonly type: OrientationType;
}

/** The orientation on the server. */
const DEFAULT_STATE: OrientationState = { angle: 0, type: "portrait-primary" };

/** Call back with the screen's orientation as it changes, until cleanup or the returned function. */
export function makeOrientation(onChange: (state: OrientationState) => void): () => void {
    // listen nowhere on the server
    if (isServer) {
        return () => {};
    }

    return makeEventListener(screen.orientation, "change", () => onChange(readOrientation()));
}

/** Follow the screen's orientation angle and type. */
export function createOrientation(): { angle: Accessor<number>; type: Accessor<OrientationType> } {
    // read the default on the server
    if (isServer) {
        return { angle: () => DEFAULT_STATE.angle, type: () => DEFAULT_STATE.type };
    }
    const initial = readOrientation();
    const [angle, setAngle] = createSignal(initial.angle, { ownedWrite: true });
    const [type, setType] = createSignal(initial.type, { ownedWrite: true });
    makeOrientation((state) => {
        setAngle(state.angle);
        setType(state.type);
    });

    return { angle, type };
}

/** Read the screen's orientation now. */
function readOrientation(): OrientationState {
    return { angle: screen.orientation.angle, type: screen.orientation.type };
}
