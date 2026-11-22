import * as style from "@destack/style";

import { overview, owed } from "../verb/story";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { bolt } from "./bolt.stylex";

/** Draw your Monday page as an app built in the browser builder and hosted on its domain. */
export function Bolt() {
    return (
        <div data-component="BoltPage" {...style.attrs(styles.page)}>
            <nav {...style.attrs(styles.nav)}>
                <b {...style.attrs(styles.brand)}>
                    <span {...style.attrs(styles.logo)}>M</span>
                    Monday
                </b>
                <span {...style.attrs(styles.navLink)}>Sign in</span>
            </nav>
            <section {...style.attrs(styles.hero)}>
                <span {...style.attrs(styles.pill)}>
                    <span {...style.attrs(styles.dot)} />
                    Invites go out Thursday
                </span>
                <b {...style.attrs(styles.title)}>Your week, from your inbox</b>
                <span {...style.attrs(styles.line)}>
                    Ines, Jonas and Mia are waiting on you. Here's what you said you'd do, and when
                    you can.
                </span>
                <span {...style.attrs(styles.form)}>
                    <span {...style.attrs(styles.input)}>Ask about your week</span>
                    <span {...style.attrs(styles.button)}>Plan my week</span>
                </span>
                <span {...style.attrs(styles.proof)}>
                    {owed.map(([initial, tint]) => (
                        <span style={{ "background-color": tint }} {...style.attrs(styles.face)}>
                            {initial}
                        </span>
                    ))}
                    <span {...style.attrs(styles.proofText)}>3 waiting on you</span>
                </span>
            </section>
            <ul {...style.attrs(styles.cards)}>
                {overview.map(([title, line]) => (
                    <li {...style.attrs(styles.card)}>
                        <b>{title}</b>
                        <span {...style.attrs(styles.cardLine)}>{line}</span>
                    </li>
                ))}
            </ul>
            <span data-component="VendorBadge" {...style.attrs(styles.badge)}>
                <Favicon icon="stackblitz" tint="#1389fd" size={12} />
                Made in Bolt
            </span>
        </div>
    );
}

/** The Bolt page styles. */
const styles = style.create({
    page: {
        backgroundColor: bolt.surface,
        backgroundImage: bolt.glow,
        color: bolt.ink,
        colorScheme: "dark",
        display: "grid",
        fontFamily: bolt.face,
        fontSize: "14px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
        position: "relative",
    },
    nav: {
        alignItems: "center",
        borderBottomColor: bolt.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        paddingBlock: "14px",
        paddingInline: { default: "32px", [site.narrow]: "16px" },
    },
    brand: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
    },
    logo: {
        alignItems: "center",
        backgroundColor: bolt.indigo,
        borderRadius: "6px",
        color: bolt.onAccent,
        display: "inline-flex",
        fontSize: "12px",
        height: "22px",
        justifyContent: "center",
        width: "22px",
    },
    navLink: {
        color: bolt.muted,
        marginInlineStart: "auto",
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
    pill: {
        alignItems: "center",
        backgroundColor: bolt.card,
        borderColor: bolt.line,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: bolt.muted,
        display: "flex",
        fontSize: "12px",
        gap: "8px",
        paddingBlock: "5px",
        paddingInline: "12px",
    },
    dot: {
        backgroundColor: bolt.indigo,
        borderRadius: "50%",
        height: "6px",
        width: "6px",
    },
    title: {
        fontSize: { default: "38px", [site.narrow]: "28px" },
        fontWeight: 700,
        letterSpacing: "-0.025em",
        lineHeight: 1.1,
        maxWidth: "520px",
    },
    line: {
        color: bolt.muted,
        maxWidth: "420px",
    },
    form: {
        display: "flex",
        gap: "8px",
        maxWidth: "420px",
        width: "100%",
    },
    input: {
        backgroundColor: bolt.card,
        borderColor: bolt.line,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: bolt.muted,
        flexGrow: 1,
        paddingBlock: "10px",
        paddingInline: "12px",
        textAlign: "start",
    },
    button: {
        backgroundColor: bolt.indigo,
        borderRadius: "8px",
        color: bolt.onAccent,
        fontWeight: 600,
        paddingBlock: "10px",
        paddingInline: "16px",
        whiteSpace: "nowrap",
    },
    proof: {
        alignItems: "center",
        display: "flex",
        gap: "4px",
    },
    face: {
        alignItems: "center",
        borderRadius: "50%",
        color: bolt.onAccent,
        display: "inline-flex",
        fontSize: "10px",
        height: "22px",
        justifyContent: "center",
        width: "22px",
    },
    proofText: {
        color: bolt.muted,
        fontSize: "12px",
        marginInlineStart: "6px",
    },
    cards: {
        alignContent: "start",
        display: "grid",
        gap: "10px",
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
        backgroundColor: bolt.card,
        borderColor: bolt.line,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        gap: "4px",
        padding: "14px",
    },
    cardLine: {
        color: bolt.muted,
        fontSize: "13px",
    },
    badge: {
        alignItems: "center",
        backgroundColor: bolt.card,
        borderColor: bolt.line,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        bottom: "14px",
        display: "flex",
        fontSize: "12px",
        gap: "6px",
        insetInlineEnd: "14px",
        paddingBlock: "5px",
        paddingInline: "10px",
        position: "absolute",
    },
});
