import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { radius, swatch } from "@destack/theme/tokens.stylex";
import { AspectRatio } from "./aspect-ratio.tsx";

/** The styles of the cover, a dusk sky drawn in place of a photo file. */
const styles = style.create({
    cover: {
        width: "100%",
        height: "100%",
        borderRadius: radius[4],
        backgroundImage: `linear-gradient(160deg, ${swatch.blueTint}, ${swatch.redTint})`,
    },
});

/** A note's cover photo at 16 by 9 whatever the width. */
export const aspectRatioCoverPhoto = defineExample({
    of: AspectRatio,
    name: "cover-photo",
    description: "a note's cover photo at 16 by 9 whatever the width",
    render: () => (
        <AspectRatio ratio={16 / 9}>
            <div role="img" aria-label="Alfama at dusk" {...style.attrs(styles.cover)} />
        </AspectRatio>
    ),
});

/** The cover cut to a square. */
export const aspectRatioSquare = defineExample({
    of: AspectRatio,
    name: "square",
    description: "the cover cut to a square",
    properties: {
        ratio: 1,
    },
    render: (properties) => (
        <AspectRatio {...properties}>
            <div role="img" aria-label="Alfama at dusk" {...style.attrs(styles.cover)} />
        </AspectRatio>
    ),
});
