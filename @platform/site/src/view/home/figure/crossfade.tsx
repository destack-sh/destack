import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";

/** Show the stacked or the destacked text in the space of the longer one. */
export function Crossfade(properties: { isOpen: boolean; stacked: string; destacked: string }) {
    return (
        <span data-component="Crossfade" {...style.attrs(styles.pair)}>
            <span
                aria-hidden={properties.isOpen ? "true" : undefined}
                {...style.attrs(styles.text, properties.isOpen && styles.hidden)}
            >
                {properties.stacked}
            </span>
            <span
                aria-hidden={properties.isOpen ? undefined : "true"}
                {...style.attrs(styles.text, !properties.isOpen && styles.hidden)}
            >
                {properties.destacked}
            </span>
        </span>
    );
}

/** The crossfade styles. */
const styles = style.create({
    pair: {
        display: "inline-grid",
    },
    text: {
        gridArea: "1 / 1",
        transition: { default: "opacity 180ms ease", [media.motionReduce]: "none" },
    },
    hidden: {
        opacity: 0,
        visibility: "hidden",
    },
});
