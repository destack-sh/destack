import { motion } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

/** Show the stacked or the destacked text, swapping only the words after those both share. */
export function Crossfade(properties: { isOpen: boolean; stacked: string; destacked: string }) {
    // keep the shared leading words still, and swap the rest in the space of the longer one
    const shared = () => sharedWordsOf(properties.stacked, properties.destacked);
    const rest = (text: string) => text.slice(shared().length);

    return (
        <span data-component="Crossfade">
            {shared()}
            <span {...style.attrs(styles.pair)}>
                <span
                    aria-hidden={properties.isOpen ? "true" : undefined}
                    {...style.attrs(styles.text, properties.isOpen && styles.stackedHidden)}
                >
                    {rest(properties.stacked)}
                </span>
                <span
                    aria-hidden={properties.isOpen ? undefined : "true"}
                    {...style.attrs(styles.text, !properties.isOpen && styles.destackedHidden)}
                >
                    {rest(properties.destacked)}
                </span>
            </span>
        </span>
    );
}

/** Return the whole words, with their trailing spaces, that both texts start with. */
function sharedWordsOf(stacked: string, destacked: string): string {
    // walk the characters both texts share
    let length = 0;
    while (length < stacked.length && stacked[length] === destacked[length]) {
        length += 1;
    }

    // cut back to the last space, so a word both texts only begin alike stays whole
    const shared = stacked.slice(0, length);

    return shared.slice(0, shared.lastIndexOf(" ") + 1);
}

/** The crossfade styles. */
const styles = style.create({
    pair: {
        display: "inline-grid",
    },
    text: {
        gridArea: "1 / 1",
        transitionDuration: motion.durationLong,
        transitionProperty: "opacity, translate, visibility",
        transitionTimingFunction: motion.easingEmphasised,
    },
    stackedHidden: {
        opacity: 0,
        translate: "0 -6px",
        visibility: "hidden",
    },
    destackedHidden: {
        opacity: 0,
        translate: "0 6px",
        visibility: "hidden",
    },
});
