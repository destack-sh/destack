import * as style from "@destack/style";

import { welcome } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { ifttt } from "./ifttt.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    back: ["m15 18-6-6 6-6"],
    share: ["M12 3v12", "m7 8 5-5 5 5", "M5 14v6h14v-6"],
    person: ["M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8z", "M4 21a8 8 0 0 1 16 0"],
    webhooks: [
        "M10 15.5a3.5 3.5 0 1 1-3-6.9",
        "m10.5 8.2 3.2 5.6",
        "M13.5 6.5a3.5 3.5 0 1 1 4.6 4.8",
        "M8.5 15.5h6.8",
        "M17 12.5a3.5 3.5 0 1 1-1.7 6.6",
    ],
} satisfies Record<string, readonly string[]>;

/** The applet's steps: the word before it, its service, what it does, and its detail. */
const steps: readonly (readonly [
    word: "If" | "Then",
    service: "webhooks" | "slack" | "gmail",
    title: string,
    detail: string,
])[] = welcome.map(([, step, detail], index) =>
    index === 0
        ? (["If", "webhooks", "Receive a web request", `${detail}, event “signup”`] as const)
        : ([
              "Then",
              index === 1 ? "slack" : "gmail",
              index === 1 ? "Post to channel" : "Send an email",
              `${step}: ${detail}`,
          ] as const),
);

/** Draw the welcome workflow as an applet: a web request from the waitlist, then two actions. */
export function Ifttt() {
    return (
        <div data-component="IftttPage" {...style.attrs(styles.page)}>
            {/* header band */}
            <header {...style.attrs(styles.band)}>
                <nav {...style.attrs(styles.nav)}>
                    <Favicon icon="ifttt" tint={ifttt.onAccent} size={48} />
                    <span {...style.attrs(styles.search)}>
                        <Mark name="search" size={14} />
                        Search for Applets, services, etc.
                    </span>
                    <span {...style.attrs(styles.navEnd)}>
                        <b {...style.attrs(styles.navLink)}>Explore</b>
                        <b {...style.attrs(styles.create)}>Create</b>
                        <span {...style.attrs(styles.me)}>F</span>
                    </span>
                </nav>
                <span {...style.attrs(styles.pills)}>
                    <span {...style.attrs(styles.pill)}>
                        <Mark name="back" size={14} />
                        Back
                    </span>
                    <span {...style.attrs(styles.pill)}>
                        <Mark name="share" size={14} />
                        Share
                    </span>
                </span>
                <div {...style.attrs(styles.hero)}>
                    <span {...style.attrs(styles.services)}>
                        <Mark name="webhooks" size={40} />
                        <Favicon icon="slack" tint={ifttt.onAccent} size={36} />
                        <Favicon icon="gmail" tint={ifttt.onAccent} size={36} />
                    </span>
                    <b {...style.attrs(styles.title)}>
                        Post new waitlist signups to #launch and welcome them by email
                    </b>
                    <span {...style.attrs(styles.meta)}>
                        <Mark name="webhooks" size={14} />
                        <b>Webhooks</b>
                        <Mark name="person" size={12} />
                        <b>1</b>
                    </span>
                </div>
            </header>

            {/* connection and steps */}
            <div {...style.attrs(styles.body)}>
                <span {...style.attrs(styles.connected)}>
                    Connected
                    <span {...style.attrs(styles.knob)} />
                </span>
                <div {...style.attrs(styles.card)}>
                    {steps.map(([word, service, title, detail]) => (
                        <span {...style.attrs(styles.step)}>
                            <b {...style.attrs(styles.word)}>{word}</b>
                            <span
                                {...style.attrs(
                                    styles.tile,
                                    service === "slack"
                                        ? styles.tileSlack
                                        : service === "gmail"
                                          ? styles.tileGmail
                                          : styles.tileWebhooks,
                                )}
                            >
                                {service === "webhooks" ? (
                                    <Mark name="webhooks" size={22} />
                                ) : (
                                    <Favicon icon={service} tint={ifttt.onAccent} size={20} />
                                )}
                            </span>
                            <span {...style.attrs(styles.stepText)}>
                                <b>{title}</b>
                                <span {...style.attrs(styles.detail)}>{detail}</span>
                            </span>
                        </span>
                    ))}
                </div>
            </div>
        </div>
    );
}

/** Draw one of the app's own glyphs in the current colour. */
function Mark(properties: { name: keyof typeof glyphs; size: number }) {
    return (
        <svg
            aria-hidden="true"
            viewBox="0 0 24 24"
            style={{ height: `${properties.size}px`, width: `${properties.size}px` }}
            {...style.attrs(styles.mark)}
        >
            {glyphs[properties.name].map((path) => (
                <path d={path} />
            ))}
        </svg>
    );
}

/** The IFTTT page styles. */
const styles = style.create({
    page: {
        backgroundColor: ifttt.surface,
        color: ifttt.ink,
        display: "grid",
        fontFamily: ifttt.face,
        fontSize: "14px",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
    },
    mark: {
        fill: "none",
        flexShrink: 0,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 2,
    },

    // header band
    band: {
        backgroundColor: ifttt.webhooks,
        borderEndEndRadius: "28px",
        borderEndStartRadius: "28px",
        color: ifttt.onAccent,
        display: "grid",
        gap: "14px",
        paddingBlockEnd: "28px",
        paddingBlockStart: "16px",
        paddingInline: { default: "28px", [site.narrow]: "16px" },
    },
    nav: {
        alignItems: "center",
        display: "flex",
        gap: "20px",
    },
    search: {
        alignItems: "center",
        borderColor: ifttt.edge,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "2px",
        display: { default: "flex", [site.compact]: "none" },
        gap: "8px",
        paddingBlock: "6px",
        paddingInline: "14px",
        width: "300px",
    },
    navEnd: {
        alignItems: "center",
        display: "flex",
        gap: "18px",
        marginInlineStart: "auto",
    },
    navLink: {
        display: { default: "inline", [site.narrow]: "none" },
    },
    create: {
        borderColor: ifttt.onAccent,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "2px",
        paddingBlock: "5px",
        paddingInline: "16px",
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        display: "inline-flex",
        fontSize: "12px",
        height: "30px",
        justifyContent: "center",
        width: "30px",
    },
    pills: {
        display: "flex",
        justifyContent: "space-between",
    },
    pill: {
        alignItems: "center",
        borderColor: ifttt.onAccent,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "2px",
        display: "inline-flex",
        fontWeight: 700,
        gap: "6px",
        paddingBlock: "5px",
        paddingInline: "14px",
    },
    hero: {
        display: "grid",
        gap: "12px",
        justifySelf: "center",
        maxWidth: "440px",
        width: "100%",
    },
    services: {
        alignItems: "center",
        display: "flex",
        gap: "16px",
    },
    title: {
        fontSize: { default: "28px", [site.narrow]: "22px" },
        fontWeight: 800,
        letterSpacing: "-0.01em",
        lineHeight: 1.1,
    },
    meta: {
        alignItems: "center",
        display: "flex",
        fontSize: "13px",
        gap: "6px",
    },

    // connection and steps
    body: {
        alignContent: "start",
        display: "grid",
        gap: "18px",
        justifyItems: "center",
        overflow: "hidden",
        paddingBlock: "18px",
        paddingInline: "16px",
    },
    connected: {
        alignItems: "center",
        backgroundColor: ifttt.switch,
        borderRadius: "999px",
        color: ifttt.onAccent,
        display: "flex",
        fontSize: "20px",
        fontWeight: 800,
        gap: "20px",
        justifyContent: "center",
        maxWidth: "440px",
        paddingBlock: "8px",
        paddingInlineEnd: "8px",
        paddingInlineStart: "28px",
        width: "100%",
    },
    knob: {
        backgroundColor: ifttt.webhooks,
        borderRadius: "50%",
        height: "40px",
        marginInlineStart: "auto",
        width: "40px",
    },
    card: {
        backgroundColor: ifttt.card,
        borderRadius: "16px",
        display: "grid",
        gap: "18px",
        maxWidth: "440px",
        paddingBlock: "18px",
        paddingInline: "20px",
        width: "100%",
    },
    step: {
        alignItems: "start",
        display: "grid",
        gap: "12px",
        gridTemplateColumns: "44px 36px minmax(0, 1fr)",
    },
    word: {
        fontSize: "18px",
        fontWeight: 800,
        paddingBlockStart: "6px",
    },
    tile: {
        alignItems: "center",
        borderRadius: "8px",
        color: ifttt.onAccent,
        display: "inline-flex",
        height: "36px",
        justifyContent: "center",
        width: "36px",
    },
    tileWebhooks: {
        backgroundColor: ifttt.webhooks,
    },
    tileSlack: {
        backgroundColor: ifttt.slack,
    },
    tileGmail: {
        backgroundColor: ifttt.gmail,
    },
    stepText: {
        display: "grid",
        gap: "2px",
        minWidth: 0,
    },
    detail: {
        color: ifttt.muted,
        fontSize: "12px",
        lineHeight: 1.4,
    },
});
