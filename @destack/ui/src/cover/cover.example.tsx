import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space, weight } from "@destack/theme/tokens.stylex";
import { Button } from "../button/index.ts";
import { Cover, CoverCenter } from "./cover.tsx";

/** The styles of the welcome screen's text and actions. */
const styles = style.create({
    brand: {
        margin: 0,
        padding: space[5],
        color: color.mutedForeground,
        fontWeight: weight.medium,
    },
    title: {
        margin: 0,
        textAlign: "center",
    },
    actions: {
        display: "flex",
        justifyContent: "center",
        padding: space[6],
    },
    missing: {
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: space[2],
        textAlign: "center",
    },
    note: {
        margin: 0,
        color: color.mutedForeground,
    },
});

/** A welcome screen with its heading centered and its actions at the bottom. */
export const coverWelcome = defineExample({
    of: Cover,
    name: "welcome",
    description: "a welcome screen with its heading centered and its actions at the bottom",
    layout: "fullscreen",
    properties: {
        minHeight: "32rem",
    },
    render: (properties) => (
        <Cover {...properties}>
            <p {...style.attrs(text.sm, styles.brand)}>Destack</p>
            <CoverCenter
                render={(attributes) => <h1 {...attributes} />}
                xstyle={[text["4xl"], styles.title]}
            >
                Take back your software
            </CoverCenter>
            <div {...style.attrs(styles.actions)}>
                <Button size="lg">Get started</Button>
            </div>
        </Cover>
    ),
});

/** A page that was not found, its message centered and the way back at the bottom. */
export const coverNotFound = defineExample({
    of: Cover,
    name: "not-found",
    description: "a page that was not found, its message centered and the way back at the bottom",
    layout: "fullscreen",
    properties: {
        minHeight: "24rem",
    },
    render: (properties) => (
        <Cover {...properties}>
            <CoverCenter xstyle={styles.missing}>
                <h1 {...style.attrs(text["2xl"], styles.title)}>This note does not exist</h1>
                <p {...style.attrs(text.sm, styles.note)}>Its owner moved or deleted it.</p>
            </CoverCenter>
            <div {...style.attrs(styles.actions)}>
                <Button variant="outline">Back to notebooks</Button>
            </div>
        </Cover>
    ),
});
