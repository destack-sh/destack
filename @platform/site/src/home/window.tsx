import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import { paper } from "../style/paper.stylex";
import { tokens } from "../style/tokens.stylex";

/** Draw a small app window: a cream card with a title bar of three lights, a title, and an optional tag. */
export function Window(properties: {
    title: string;
    tag?: string;
    isBranch?: boolean;
    style?: stylex.Styles;
    children: JSX.Element;
}) {
    return (
        <div {...stylex.attrs(paper.card, styles.window, properties.style)}>
            <p {...stylex.attrs(styles.chrome)}>
                <span aria-hidden="true" {...stylex.attrs(styles.lights)} />
                {properties.title}
                {properties.tag !== undefined && (
                    <em
                        {...stylex.attrs(styles.tag, properties.isBranch === true && styles.branch)}
                    >
                        {properties.tag}
                    </em>
                )}
            </p>
            {properties.children}
        </div>
    );
}

/** Caption a window with its step: the verb in bold, then the rest as written. */
export function Caption(properties: { verb: string; rest: string }) {
    return (
        <p {...stylex.attrs(styles.caption)}>
            <b {...stylex.attrs(styles.verb)}>{properties.verb}</b>
            {properties.rest}
        </p>
    );
}

/** The window styles. */
const styles = stylex.create({
    caption: {
        margin: 0,
    },
    verb: {
        fontWeight: 600,
    },
    window: {
        display: "grid",
        gridTemplateRows: "2.125rem minmax(0, 1fr)",
        overflow: "hidden",
    },
    chrome: {
        alignItems: "center",
        borderBottomColor: tokens.signalInk,
        borderBottomStyle: "solid",
        borderBottomWidth: "2px",
        display: "flex",
        fontFamily: tokens.monoFont,
        fontSize: "0.74rem",
        fontWeight: 600,
        gap: "0.5rem",
        margin: 0,
        paddingInline: "0.75rem",
    },
    lights: {
        backgroundImage: `radial-gradient(circle, transparent 2.2px, ${tokens.signalInk} 2.6px, ${tokens.signalInk} 3.6px, transparent 4px)`,
        backgroundPosition: "0 50%",
        backgroundRepeat: "repeat-x",
        backgroundSize: "10px 8px",
        flexShrink: 0,
        height: "8px",
        width: "28px",
    },
    tag: {
        borderColor: tokens.signalInk,
        borderStyle: "solid",
        borderWidth: "1px",
        fontStyle: "normal",
        fontWeight: 400,
        marginLeft: "auto",
        paddingBlock: "3px",
        paddingInline: "7px",
    },
    branch: {
        backgroundColor: tokens.signal,
    },
});
