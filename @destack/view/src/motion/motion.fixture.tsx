import { createSignal } from "solid-js";
import { createReducedMotion } from "./motion.ts";

/** Show whether the theme keeps the paragraph's motion still. */
export function MotionState() {
    // follow the paragraph's motion once it mounts
    const [element, setElement] = createSignal<Element>();
    const isReduced = createReducedMotion(element);

    return <p ref={setElement}>{isReduced() ? "still" : "moving"}</p>;
}
