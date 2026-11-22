import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { size, space } from "@destack/theme/tokens.stylex";
import { Skeleton } from "./skeleton.tsx";

/** The sizes of a loading list row. */
const styles = style.create({
    row: { display: "flex", alignItems: "center", gap: space[3], width: "100%" },
    avatar: { width: size[3], height: size[3] },
    line: { flex: 1, height: space[4] },
    card: { display: "flex", flexDirection: "column", gap: space[2], width: "100%" },
    heading: { width: "40%", height: space[5] },
    text: { height: space[4] },
    short: { width: "60%", height: space[4] },
});

/** A list row's avatar and title while the row loads. */
export const skeletonLoadingRow = defineExample({
    of: Skeleton,
    name: "loading-row",
    description: "a list row's avatar and title while the row loads",
    render: () => (
        <div aria-busy="true" {...style.attrs(styles.row)}>
            <Skeleton xstyle={styles.avatar} />
            <Skeleton xstyle={styles.line} />
        </div>
    ),
});

/** A card's title and three lines of text while the card loads. */
export const skeletonLoadingCard = defineExample({
    of: Skeleton,
    name: "loading-card",
    description: "a card's title and three lines of text while the card loads",
    render: () => (
        <div aria-busy="true" {...style.attrs(styles.card)}>
            <Skeleton xstyle={styles.heading} />
            <Skeleton xstyle={styles.text} />
            <Skeleton xstyle={styles.text} />
            <Skeleton xstyle={styles.short} />
        </div>
    ),
});
