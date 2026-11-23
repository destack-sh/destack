import * as style from "@destack/style";
import { type Accessor, createSignal, type JSX } from "@destack/view";
import { createElementSize } from "@destack/view/primitives/resize-observer";

/** The style of a length's element: out of sight and out of the flow, as long as the length. */
const styles = style.create({
    element: (length: string) => ({
        position: "absolute",
        insetBlockStart: 0,
        insetInlineStart: 0,
        inlineSize: length,
        blockSize: 0,
        visibility: "hidden",
        pointerEvents: "none",
    }),
});

/** A CSS length read in pixels as the theme changes, from an element as long as it. */
export class Length {
    /** The element that measures the length, rendered inside the element whose theme sets it. */
    readonly element: JSX.Element;
    /** The length in pixels, none until the element is measured. */
    readonly pixels: Accessor<number | undefined>;

    /** Measure a length, such as a size token. */
    constructor(length: Accessor<string>) {
        // follow the element's width as the length or the theme changes
        const [element, setElement] = createSignal<HTMLElement | undefined>(undefined, {
            ownedWrite: true,
        });
        const size = createElementSize(element);
        this.pixels = () => size.width ?? undefined;
        this.element = (
            <div
                data-slot="length"
                aria-hidden="true"
                ref={setElement}
                {...style.attrs(styles.element(length()))}
            />
        );
    }
}
