import * as style from "@destack/style";

import { welcome } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { ifttt } from "./ifttt.stylex";

/** The service's wordmark, its block letters on a 203 by 52 grid. */
const WORDMARK =
    "M109.4 0H68.1V15.6H80.4V52.4H97.1V15.6H109.4V0ZM156.2 0H115V15.6H127.2V52.4H144V15.6H156.2V0ZM203.1 0H161.8V15.6H174.1V52.4H190.8V15.6H203.1V0ZM16.7 0H0V52.4H16.7V0ZM62.5 0H24.6V52.4H41.3V37.9H55.8V21.2H41.3V15.6H62.5V0Z";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    back: ["m15 18-6-6 6-6"],
    share: ["M12 3v12", "m7 8 5-5 5 5", "M5 14v6h14v-6"],
    person: ["M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8z", "M4 21a8 8 0 0 1 16 0"],
    bolt: ["M13 3 5 14h6l-1 7 8-11h-6z"],
    webhooks: [
        "M12 10a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "M6.5 20a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "M17.5 20a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "M10.4 9.5 8 14",
        "M13.6 9.5 16 14",
        "M9.5 17h5",
    ],
} satisfies Record<string, readonly string[]>;

/** The applet's steps: its word, service, title, detail and ingredients. */
const steps: readonly (readonly [
    word: "If" | "Then",
    service: "webhooks" | "slack" | "gmail",
    title: string,
    detail: string,
    ingredients: readonly string[],
])[] = welcome.map(([, step, detail], index) =>
    index === 0
        ? ([
              "If",
              "webhooks",
              "Receive a web request",
              `${detail}, event “signup”`,
              ["EventName", "Value1", "OccurredAt"],
          ] as const)
        : ([
              "Then",
              index === 1 ? "slack" : "gmail",
              index === 1 ? "Post to channel" : "Send an email",
              `${step}: ${detail}`,
              [],
          ] as const),
);

/** Draw the welcome workflow as an applet: a web request from the waitlist, then two actions. */
export function Ifttt() {
    return (
        <div data-component="IftttPage" {...style.attrs(styles.page)}>
            {/* header band */}
            <header {...style.attrs(styles.band)}>
                <nav {...style.attrs(styles.nav)}>
                    <svg aria-label="IFTTT" viewBox="0 0 203 52" {...style.attrs(styles.wordmark)}>
                        <path d={WORDMARK} />
                    </svg>
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
                    <b {...style.attrs(styles.pill)}>
                        <Mark name="back" size={14} />
                        Back
                    </b>
                    <b {...style.attrs(styles.pill)}>
                        <Mark name="share" size={14} />
                        Share
                    </b>
                </span>
                <div {...style.attrs(styles.hero)}>
                    <span {...style.attrs(styles.services)}>
                        <Mark name="webhooks" size={46} />
                        <Favicon icon="slack" tint={ifttt.onAccent} size={42} />
                        <Favicon icon="gmail" tint={ifttt.onAccent} size={42} />
                    </span>
                    <b {...style.attrs(styles.title)}>
                        Post new waitlist signups to #launch and welcome them by email
                    </b>
                    <span {...style.attrs(styles.meta)}>
                        <Mark name="webhooks" size={14} />
                        <b>Webhooks</b>
                        <Mark name="person" size={11} />
                        <b>1</b>
                    </span>
                </div>
            </header>

            {/* connection, about and steps */}
            <div {...style.attrs(styles.body)}>
                <span {...style.attrs(styles.connected)}>
                    Connected
                    <span {...style.attrs(styles.knob)} />
                </span>
                <div {...style.attrs(styles.about)}>
                    <b {...style.attrs(styles.aboutTitle)}>About this Applet</b>
                    <span {...style.attrs(styles.aboutText)}>
                        Each new waitlist signup posts to #launch in Slack and gets a welcome email
                        from Gmail.
                    </span>
                </div>
                <div {...style.attrs(styles.card)}>
                    {steps.map(([word, service, title, detail, ingredients], index) => (
                        <div {...style.attrs(styles.step)}>
                            <b {...style.attrs(styles.word)}>{word}</b>
                            <span {...style.attrs(styles.rail)}>
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
                                {index < steps.length - 1 && <span {...style.attrs(styles.line)} />}
                            </span>
                            <span {...style.attrs(styles.stepText)}>
                                <b {...style.attrs(styles.stepTitle)}>{title}</b>
                                {word === "If" && (
                                    <span {...style.attrs(styles.chip)}>
                                        <Mark name="bolt" size={11} />
                                        Realtime trigger
                                    </span>
                                )}
                                <span {...style.attrs(styles.detail)}>{detail}</span>
                                {ingredients.length > 0 && (
                                    <span {...style.attrs(styles.ingredients)}>
                                        <span {...style.attrs(styles.detail)}>
                                            Trigger ingredients
                                        </span>
                                        <span {...style.attrs(styles.tags)}>
                                            {ingredients.map((ingredient) => (
                                                <span {...style.attrs(styles.tag)}>
                                                    {ingredient}
                                                </span>
                                            ))}
                                        </span>
                                    </span>
                                )}
                            </span>
                        </div>
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
        fontWeight: 700,
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
        strokeWidth: 2.25,
    },

    // header band
    band: {
        backgroundColor: ifttt.webhooks,
        borderEndEndRadius: "28px",
        borderEndStartRadius: "28px",
        color: ifttt.onAccent,
        display: "grid",
        gap: "16px",
        paddingBlockEnd: "30px",
        paddingBlockStart: "16px",
        paddingInline: { default: "24px", [site.narrow]: "16px" },
    },
    nav: {
        alignItems: "center",
        display: "flex",
        gap: "18px",
    },
    wordmark: {
        fill: "currentColor",
        flexShrink: 0,
        height: "18px",
        width: "70px",
    },
    search: {
        alignItems: "center",
        borderColor: ifttt.edge,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "2px",
        display: { default: "flex", [site.compact]: "none" },
        fontSize: "12px",
        gap: "8px",
        paddingBlock: "5px",
        paddingInline: "12px",
        width: "240px",
    },
    navEnd: {
        alignItems: "center",
        display: "flex",
        fontWeight: 900,
        gap: "16px",
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
        paddingBlock: "4px",
        paddingInline: "14px",
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        display: "inline-flex",
        fontSize: "12px",
        height: "28px",
        justifyContent: "center",
        width: "28px",
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
        fontWeight: 900,
        gap: "6px",
        paddingBlock: "5px",
        paddingInline: "14px",
    },
    hero: {
        display: "grid",
        gap: "14px",
        justifySelf: "center",
        maxWidth: "380px",
        width: "100%",
    },
    services: {
        alignItems: "center",
        display: "flex",
        gap: "20px",
    },
    title: {
        fontSize: { default: "27px", [site.narrow]: "22px" },
        fontWeight: 900,
        lineHeight: 1,
    },
    meta: {
        alignItems: "center",
        display: "flex",
        fontSize: "12px",
        fontWeight: 900,
        gap: "5px",
    },

    // connection, about and steps
    body: {
        alignContent: "start",
        display: "grid",
        gap: "20px",
        justifyItems: "center",
        overflow: "hidden",
        paddingBlock: "20px",
        paddingInline: "16px",
    },
    connected: {
        alignItems: "center",
        backgroundColor: ifttt.switch,
        borderRadius: "999px",
        color: ifttt.onAccent,
        display: "flex",
        fontSize: "22px",
        fontWeight: 900,
        justifyContent: "center",
        maxWidth: "380px",
        paddingBlock: "5px",
        paddingInlineEnd: "5px",
        paddingInlineStart: "30px",
        width: "100%",
    },
    knob: {
        backgroundColor: ifttt.webhooks,
        borderRadius: "50%",
        height: "42px",
        marginInlineStart: "auto",
        width: "42px",
    },
    about: {
        display: "grid",
        gap: "6px",
        maxWidth: "380px",
        width: "100%",
    },
    aboutTitle: {
        fontSize: "16px",
        fontWeight: 900,
    },
    aboutText: {
        fontSize: "14px",
        lineHeight: 1.35,
    },
    card: {
        backgroundColor: ifttt.card,
        borderRadius: "14px",
        display: "grid",
        maxWidth: "380px",
        paddingBlock: "16px",
        paddingInline: "16px",
        width: "100%",
    },
    step: {
        alignItems: "start",
        columnGap: "12px",
        display: "grid",
        gridTemplateColumns: "40px 32px minmax(0, 1fr)",
    },
    word: {
        fontSize: "20px",
        fontWeight: 900,
        paddingBlockStart: "2px",
    },
    rail: {
        alignSelf: "stretch",
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        justifyItems: "center",
    },
    line: {
        backgroundColor: ifttt.tag,
        minHeight: "14px",
        width: "2px",
    },
    tile: {
        alignItems: "center",
        borderRadius: "8px",
        color: ifttt.onAccent,
        display: "inline-flex",
        height: "32px",
        justifyContent: "center",
        width: "32px",
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
        gap: "5px",
        justifyItems: "start",
        minWidth: 0,
        paddingBlockEnd: "14px",
    },
    stepTitle: {
        fontSize: "14px",
        fontWeight: 900,
        lineHeight: 1.2,
    },
    chip: {
        alignItems: "center",
        backgroundColor: ifttt.tag,
        borderRadius: "4px",
        display: "inline-flex",
        fontSize: "11px",
        gap: "4px",
        paddingBlock: "2px",
        paddingInline: "6px",
    },
    detail: {
        color: ifttt.muted,
        fontSize: "11px",
        lineHeight: 1.4,
    },
    ingredients: {
        borderTopColor: ifttt.tag,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        display: "grid",
        gap: "5px",
        paddingBlockStart: "8px",
        width: "100%",
    },
    tags: {
        display: "flex",
        flexWrap: "wrap",
        gap: "5px",
    },
    tag: {
        backgroundColor: ifttt.tag,
        borderRadius: "4px",
        fontSize: "11px",
        paddingBlock: "2px",
        paddingInline: "6px",
    },
});
