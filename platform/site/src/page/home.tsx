import * as stylex from "@destack/style";

import { Cover } from "../home/cover";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";

/// Render the public Destack homepage.
export function HomePage() {
    return (
        <Shell isHome>
            <Seo
                description={
                    "Destack is a universal software engine for building complete " +
                    "software systems."
                }
            />

            <article {...stylex.attrs(styles.article)}>
                <Cover />
            </article>
        </Shell>
    );
}

const styles = stylex.create({
    article: {
        display: "grid",
        minHeight: 0,
    },
});
