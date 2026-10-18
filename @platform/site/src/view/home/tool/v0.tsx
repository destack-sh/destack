import * as style from "@destack/style";

import { joiners, perks } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { v0 } from "./v0.stylex";

/** Draw your waitlist as an app the UI generator made and deployed on the hosting platform. */
export function V0() {
    return (
        <div data-component="V0Page" {...style.attrs(styles.page)}>
            <nav {...style.attrs(styles.nav)}>
                <b>Destack</b>
                <span {...style.attrs(styles.links)}>
                    <span>Features</span>
                    <span>FAQ</span>
                </span>
                <span {...style.attrs(styles.navButton)}>Join waitlist</span>
            </nav>
            <section {...style.attrs(styles.hero)}>
                <span {...style.attrs(styles.badgeLine)}>Opens Friday, October 24 →</span>
                <b {...style.attrs(styles.title)}>The software you'll actually own.</b>
                <span {...style.attrs(styles.line)}>
                    Destack opens on Friday. Leave your email and we'll send you an invite.
                </span>
                <span {...style.attrs(styles.form)}>
                    <span {...style.attrs(styles.input)}>name@example.com</span>
                    <span {...style.attrs(styles.button)}>Join the waitlist</span>
                </span>
                <span {...style.attrs(styles.proof)}>
                    <span {...style.attrs(styles.faces)}>
                        {joiners.map(([initial]) => (
                            <span {...style.attrs(styles.face)}>{initial}</span>
                        ))}
                    </span>
                    Join 1,840 others
                </span>
            </section>
            <ul {...style.attrs(styles.cards)}>
                {perks.map(([title, line]) => (
                    <li {...style.attrs(styles.card)}>
                        <b>{title}</b>
                        <span {...style.attrs(styles.cardLine)}>{line}</span>
                    </li>
                ))}
            </ul>
            <span data-component="VendorBadge" {...style.attrs(styles.badge)}>
                <Favicon icon="v0" tint="#000000" size={14} />
                Built with v0
            </span>
        </div>
    );
}

/** The v0 page styles. */
const styles = style.create({
    page: {
        backgroundColor: v0.surface,
        color: v0.ink,
        display: "grid",
        fontFamily: v0.face,
        fontSize: "14px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
        position: "relative",
    },
    nav: {
        alignItems: "center",
        borderBottomColor: v0.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "24px",
        paddingBlock: "14px",
        paddingInline: { default: "32px", [site.narrow]: "16px" },
    },
    links: {
        color: v0.quiet,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "20px",
        marginInlineStart: "auto",
    },
    navButton: {
        backgroundColor: v0.ink,
        borderRadius: "6px",
        color: v0.surface,
        fontWeight: 500,
        marginInlineStart: { default: "0", [site.narrow]: "auto" },
        paddingBlock: "7px",
        paddingInline: "12px",
    },
    hero: {
        alignItems: "center",
        display: "flex",
        flexDirection: "column",
        gap: "16px",
        paddingBlockEnd: "28px",
        paddingBlockStart: "40px",
        paddingInline: "24px",
        textAlign: "center",
    },
    badgeLine: {
        borderColor: v0.line,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: v0.quiet,
        fontSize: "12px",
        paddingBlock: "4px",
        paddingInline: "12px",
    },
    title: {
        fontSize: { default: "40px", [site.narrow]: "28px" },
        fontWeight: 700,
        letterSpacing: "-0.04em",
        lineHeight: 1.05,
        maxWidth: "520px",
    },
    line: {
        color: v0.quiet,
        maxWidth: "420px",
    },
    form: {
        display: "flex",
        gap: "8px",
        maxWidth: "400px",
        width: "100%",
    },
    input: {
        borderColor: v0.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: v0.quiet,
        flexGrow: 1,
        paddingBlock: "9px",
        paddingInline: "12px",
        textAlign: "start",
    },
    button: {
        backgroundColor: v0.ink,
        borderRadius: "6px",
        color: v0.surface,
        fontWeight: 500,
        paddingBlock: "9px",
        paddingInline: "14px",
        whiteSpace: "nowrap",
    },
    proof: {
        alignItems: "center",
        color: v0.quiet,
        display: "flex",
        fontSize: "13px",
        gap: "10px",
    },
    faces: {
        display: "flex",
    },
    face: {
        alignItems: "center",
        backgroundColor: v0.muted,
        borderColor: v0.surface,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "2px",
        display: "inline-flex",
        fontSize: "11px",
        height: "26px",
        justifyContent: "center",
        marginInlineEnd: "-8px",
        width: "26px",
    },
    cards: {
        alignContent: "start",
        display: "grid",
        gap: "12px",
        gridTemplateColumns: {
            default: "repeat(3, minmax(0, 1fr))",
            [site.narrow]: "minmax(0, 1fr)",
        },
        listStyle: "none",
        margin: 0,
        paddingBlock: 0,
        paddingInline: { default: "32px", [site.narrow]: "16px" },
    },
    card: {
        borderColor: v0.line,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        gap: "4px",
        padding: "16px",
    },
    cardLine: {
        color: v0.quiet,
        fontSize: "13px",
    },
    badge: {
        alignItems: "center",
        backgroundColor: v0.surface,
        borderColor: v0.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        bottom: "14px",
        display: "flex",
        fontSize: "12px",
        fontWeight: 500,
        gap: "6px",
        insetInlineEnd: "14px",
        paddingBlock: "5px",
        paddingInline: "10px",
        position: "absolute",
    },
});
