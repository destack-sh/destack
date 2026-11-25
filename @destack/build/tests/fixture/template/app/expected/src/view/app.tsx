import { MARK } from "@destack/theme/mark";
import * as style from "@destack/style";
import { swatch } from "@destack/theme/tokens.stylex";
import type { JSX } from "@destack/view";

/** The mark as a data URL an image loads. */
const MARK_URL = `data:image/svg+xml,${encodeURIComponent(MARK)}`;

/** The page's layout: the brand's orange, the mark centred on it. */
const styles = style.create({
    page: {
        display: "grid",
        placeItems: "center",
        minBlockSize: "100dvh",
        backgroundColor: swatch.orangeSolid,
    },
    mark: {
        inlineSize: "6rem",
        blockSize: "6rem",
    },
});

/** Render an empty page with Destack's mark. */
export default function App(): JSX.Element {
    return (
        <main {...style.attrs(styles.page)}>
            <img src={MARK_URL} alt="Destack" {...style.attrs(styles.mark)} />
        </main>
    );
}
