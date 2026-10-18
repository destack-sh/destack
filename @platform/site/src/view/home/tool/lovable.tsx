import * as style from "@destack/style";

import { joiners, perks } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { lovable } from "./lovable.stylex";

/** Draw your waitlist as an app generated from a prompt and published on the builder's domain. */
export function Lovable() {
    return (
        <div data-component="LovablePage" {...style.attrs(styles.page)}>
            {/* navigation */}
            <nav {...style.attrs(styles.nav)}>
                <b {...style.attrs(styles.brand)}>
                    <span {...style.attrs(styles.logo)} />
                    Destack
                </b>
                <span {...style.attrs(styles.links)}>
                    <span>How it works</span>
                    <span>Pricing</span>
                </span>
                <span {...style.attrs(styles.navButton)}>Get early access</span>
            </nav>

            {/* hero */}
            <section {...style.attrs(styles.hero)}>
                <span {...style.attrs(styles.pill)}>✨ Launching Friday, October 24</span>
                <b {...style.attrs(styles.title)}>
                    Own your software. <span {...style.attrs(styles.gradientText)}>Finally.</span>
                </b>
                <span {...style.attrs(styles.line)}>
                    Destack opens on Friday. Join the waitlist and we'll send your invite.
                </span>
                <span {...style.attrs(styles.form)}>
                    <span {...style.attrs(styles.input)}>Enter your email</span>
                    <span {...style.attrs(styles.button)}>Join the waitlist</span>
                </span>
                <span {...style.attrs(styles.proof)}>
                    <span {...style.attrs(styles.faces)}>
                        {joiners.map(([initial, tint]) => (
                            <span
                                style={{ "background-color": tint }}
                                {...style.attrs(styles.face)}
                            >
                                {initial}
                            </span>
                        ))}
                    </span>
                    1,840 people already joined
                </span>
            </section>

            {/* perks */}
            <ul {...style.attrs(styles.cards)}>
                {perks.map(([title, line]) => (
                    <li {...style.attrs(styles.card)}>
                        <b>{title}</b>
                        <span {...style.attrs(styles.cardLine)}>{line}</span>
                    </li>
                ))}
            </ul>

            <span data-component="VendorBadge" {...style.attrs(styles.badge)}>
                <Favicon icon="lovable" tint="#ff4f6d" size={13} />
                Edit with Lovable
            </span>
        </div>
    );
}

/** The Lovable page styles. */
const styles = style.create({
    page: {
        backgroundColor: lovable.surface,
        color: lovable.ink,
        display: "grid",
        fontFamily: lovable.face,
        fontSize: "14px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
        position: "relative",
    },
    nav: {
        alignItems: "center",
        display: "flex",
        gap: "24px",
        paddingBlock: "16px",
        paddingInline: { default: "40px", [site.narrow]: "16px" },
    },
    brand: {
        alignItems: "center",
        display: "flex",
        fontSize: "16px",
        gap: "8px",
    },
    logo: {
        backgroundImage: lovable.glow,
        borderRadius: "8px",
        height: "22px",
        width: "22px",
    },
    links: {
        color: lovable.muted,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "20px",
        marginInlineStart: "auto",
    },
    navButton: {
        backgroundColor: lovable.ink,
        borderRadius: "999px",
        color: lovable.surface,
        fontWeight: 600,
        marginInlineStart: { default: "0", [site.narrow]: "auto" },
        paddingBlock: "8px",
        paddingInline: "16px",
    },
    hero: {
        alignItems: "center",
        display: "flex",
        flexDirection: "column",
        gap: "16px",
        paddingBlockEnd: "28px",
        paddingBlockStart: "36px",
        paddingInline: "24px",
        textAlign: "center",
    },
    pill: {
        backgroundColor: lovable.card,
        borderColor: lovable.line,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: lovable.muted,
        fontSize: "12px",
        paddingBlock: "5px",
        paddingInline: "12px",
    },
    title: {
        fontSize: { default: "40px", [site.narrow]: "30px" },
        fontWeight: 800,
        letterSpacing: "-0.03em",
        lineHeight: 1.05,
        maxWidth: "520px",
    },
    gradientText: {
        backgroundClip: "text",
        backgroundImage: lovable.glow,
        color: "transparent",
    },
    line: {
        color: lovable.muted,
        fontSize: "15px",
        maxWidth: "420px",
    },
    form: {
        backgroundColor: lovable.card,
        borderColor: lovable.line,
        borderRadius: "14px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        gap: "6px",
        maxWidth: "420px",
        padding: "6px",
        width: "100%",
    },
    input: {
        color: lovable.muted,
        flexGrow: 1,
        paddingBlock: "10px",
        paddingInline: "12px",
        textAlign: "start",
    },
    button: {
        backgroundImage: lovable.glow,
        borderRadius: "10px",
        color: lovable.onAccent,
        fontWeight: 600,
        paddingBlock: "10px",
        paddingInline: "16px",
        whiteSpace: "nowrap",
    },
    proof: {
        alignItems: "center",
        color: lovable.muted,
        display: "flex",
        fontSize: "13px",
        gap: "10px",
    },
    faces: {
        display: "flex",
    },
    face: {
        alignItems: "center",
        borderColor: lovable.surface,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "2px",
        color: lovable.onAccent,
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
        paddingInline: { default: "40px", [site.narrow]: "16px" },
    },
    card: {
        backgroundColor: lovable.card,
        borderColor: lovable.line,
        borderRadius: "14px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        gap: "4px",
        padding: "16px",
    },
    cardLine: {
        color: lovable.muted,
        fontSize: "13px",
    },
    badge: {
        alignItems: "center",
        backgroundColor: lovable.badge,
        borderRadius: "8px",
        bottom: "14px",
        color: lovable.onAccent,
        display: "flex",
        fontSize: "12px",
        fontWeight: 500,
        gap: "6px",
        insetInlineEnd: "14px",
        paddingBlock: "6px",
        paddingInline: "10px",
        position: "absolute",
    },
});
