import * as style from "@destack/style";
import { size, space } from "@destack/theme/tokens.stylex";
import type { JSX } from "@solidjs/web";
import { Skeleton } from "./skeleton.tsx";

/** The sizes of a loading list row. */
const styles = style.create({
    row: { display: "flex", alignItems: "center", gap: space[3] },
    avatar: { width: size[3], height: size[3] },
    line: { flex: 1, height: space[4] },
});

/** Show a list row's avatar and title while the row loads. */
export function SkeletonExample(): JSX.Element {
    return (
        <div aria-busy="true" {...style.attrs(styles.row)}>
            <Skeleton style={styles.avatar} />
            <Skeleton style={styles.line} />
        </div>
    );
}
