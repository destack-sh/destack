import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { size, space } from "@destack/theme/tokens.stylex";
import { Skeleton } from "./skeleton.tsx";

/** The sizes of a loading list row. */
const styles = style.create({
    row: { display: "flex", alignItems: "center", gap: space[3] },
    avatar: { width: size[3], height: size[3] },
    line: { flex: 1, height: space[4] },
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
