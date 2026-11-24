import * as style from "@destack/style";
import { createEffect, createSignal, type JSX, Show } from "@destack/view";
import { type Anchor, type AnchorBox, Position } from "./position.ts";

/** Place an overlay beside an element, or beside a box through an element that stands there. */
export function PositionAnchor(properties: {
    /** What the overlay is placed beside, its trigger when absent. */
    readonly anchor: Anchor | undefined;
    /** Set the element the overlay is placed beside. */
    readonly onAnchor: (element: HTMLElement | undefined) => void;
}): JSX.Element {
    // follow the element, or the one that stands where the box is
    const box = (): AnchorBox | undefined =>
        isElement(properties.anchor) ? undefined : properties.anchor;
    const [stand, setStand] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    createEffect(
        (): HTMLElement | undefined => {
            const { anchor } = properties;

            return isElement(anchor) ? anchor : anchor === undefined ? undefined : stand();
        },
        (element, before) => {
            if (element !== before) {
                properties.onAnchor(element);
            }
        },
    );

    return (
        <Show when={box()}>
            {(current) => (
                <span
                    aria-hidden="true"
                    data-slot="position-anchor"
                    ref={(element) => setStand(element)}
                    {...style.attributes([Position.box(current())])}
                />
            )}
        </Show>
    );
}

/** Report whether an anchor is an element, also where a page renders without one. */
function isElement(anchor: Anchor | undefined): anchor is HTMLElement {
    return anchor !== undefined && "nodeType" in anchor;
}
