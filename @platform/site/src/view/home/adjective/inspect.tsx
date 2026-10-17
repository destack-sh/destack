import * as style from "@destack/style";
import { present } from "@destack/schema";
import { color, font, shadow, stroke } from "@destack/theme/tokens.stylex";
import { createSignal, type JSX } from "@destack/view";
import { createTimer } from "@destack/view/primitives/timer";

import { palette } from "../../palette.stylex";
import { screen } from "../../layout/screen.stylex";
import { chrome } from "../figure/chrome.stylex";
import { browserStyles } from "../figure/browser";
import { Fade } from "../figure/fade";
import { Favicon, Glyph } from "../figure/glyph";
import { ObservabilityIssue } from "./observability";
import { batchRequests, ResendLogs, SentryIssue, SupabaseQuery, TemporalRun } from "./console";
import { ChatGPTTasks, GitHubRun, NotionAutomations } from "./rented";

/** A rented console you open to look at launch day. */
type Console = "chatgpt" | "notion" | "sentry" | "resend" | "temporal" | "supabase" | "github";

/** The consoles you rent, as tabs: key, logo, title, address and page. */
const consoles: readonly (readonly [
    key: Console,
    logo: string,
    title: string,
    url: string,
    page: () => JSX.Element,
])[] = [
    ["chatgpt", "openai", "Scheduled tasks · ChatGPT", "chatgpt.com/tasks", ChatGPTTasks],
    ["notion", "notion", "Waitlist · Notion", "notion.so/symbol/waitlist", NotionAutomations],
    [
        "sentry",
        "sentry.png",
        "ResendError: 429 · WAITLIST-4812",
        "symbol.sentry.io/issues/4812/",
        SentryIssue,
    ],
    [
        "resend",
        "resend.svg",
        "Logs · Resend",
        "resend.com/logs",
        () => <ResendLogs requests={batchRequests} />,
    ],
    [
        "temporal",
        "temporal.png",
        "invite-batch-3 · Workflows",
        "cloud.temporal.io/namespaces/default/workflows/invite-batch-3",
        TemporalRun,
    ],
    [
        "supabase",
        "supabase.svg",
        "SQL Editor · waitlist",
        "supabase.com/dashboard/project/waitlist/sql",
        SupabaseQuery,
    ],
    [
        "github",
        "github",
        "Batch invites through Resend · Actions",
        "github.com/symbol/waitlist-app/actions/runs/87",
        GitHubRun,
    ],
];

/** The milliseconds each console shows before the browser turns to the next. */
const TURN_TIME = 2400;

/** What each console tells you about launch day, in a line. */
const findings: Readonly<Record<Console, string>> = {
    chatgpt: "a task can't reach the waitlist database",
    notion: "Maya asks who approved this batch",
    sentry: "ResendError 429 at batch 3",
    resend: "170 sends rejected with 429",
    temporal: "sendInvites failed at 230",
    supabase: "1,840 still waiting",
    github: "a test wants batches to wait out the limit",
};

/** The questions none of the consoles answers. */
const questions: readonly string[] = ["Who approved batch 3?", "Did the waitlist get marked?"];

/** The colours of the vendor icons drawn from the glyph set. */
const tints: Readonly<Record<string, string>> = {
    openai: "#10a37f",
    github: "#181717",
    notion: "#191919",
};

/** Draw launch day: across rented consoles that each see a sliver, or on your own dashboard. */
export function Inspector(properties: { isOpen: boolean }) {
    return (
        <Fade
            isOpen={properties.isOpen}
            xstyle={styles.fade}
            isLarge
            component={properties.isOpen ? "SpaceDashboard" : "RentedConsoles"}
        >
            {properties.isOpen ? <SpaceDashboard /> : <RentedConsoles />}
        </Fade>
    );
}

/** Draw the rented consoles in a browser, a tab each turning on its own, and what each told you. */
function RentedConsoles() {
    // turn to the next console every page time until the reader opens one
    const [turned, setTurned] = createSignal(0);
    const [opened, setOpened] = createSignal<Console>();
    createTimer(
        () => setTurned((current) => current + 1),
        () => (opened() === undefined ? TURN_TIME : false),
        setInterval,
    );
    const index = () => {
        const chosen = opened();

        return chosen === undefined
            ? turned() % consoles.length
            : consoles.findIndex(([key]) => key === chosen);
    };
    const shown = () => present(consoles[index()], "shown console");

    // collect what every console seen so far told you
    const seen = () =>
        consoles.filter(([key], position) => position <= turned() || key === opened());

    return (
        <div data-component="Browser" {...style.attrs(browserStyles.browser, styles.browser)}>
            <div data-component="TabStrip" {...style.attrs(browserStyles.strip)}>
                <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                {consoles.map(([key, logo, title]) => (
                    <span
                        data-component="Tab"
                        title={title}
                        onClick={() => setOpened(key)}
                        {...style.attrs(
                            browserStyles.tab,
                            browserStyles.tabEven,
                            key === shown()[0] && browserStyles.tabOn,
                            browserStyles.tabSteady,
                        )}
                    >
                        <TabMark logo={logo} />
                        <span {...style.attrs(browserStyles.tabTitle)}>{title}</span>
                    </span>
                ))}
            </div>
            <Address url={shown()[3]} />
            <div {...style.attrs(browserStyles.viewport, styles.viewport)}>
                <div {...style.attrs(styles.scaled)}>{shown()[4]()}</div>

                {/* what each console told you, and what none of them answers */}
                <aside data-component="Investigation" {...style.attrs(styles.investigation)}>
                    <span {...style.attrs(styles.found)}>
                        <span {...style.attrs(styles.investigationLabel)}>
                            Found in {seen().length} of {consoles.length} tabs
                        </span>
                        {seen().map(([key, logo]) => (
                            <span {...style.attrs(styles.finding)}>
                                <TabMark logo={logo} />
                                {findings[key]}
                            </span>
                        ))}
                    </span>
                    <span {...style.attrs(styles.open)}>
                        <span {...style.attrs(styles.investigationLabel)}>Still open</span>
                        {questions.map((question) => (
                            <span {...style.attrs(styles.question)}>
                                <span {...style.attrs(styles.questionMark)}>?</span>
                                {question}
                            </span>
                        ))}
                    </span>
                </aside>
            </div>
        </div>
    );
}

/** Draw a tab's mark: a vendor's logo file, or its icon in its colour. */
function TabMark(properties: { logo: string }) {
    return (
        <>
            {properties.logo.includes(".") ? (
                <img alt="" src={`/logos/${properties.logo}`} {...style.attrs(styles.favicon)} />
            ) : (
                <Favicon icon={properties.logo} tint={tints[properties.logo] ?? palette.ink} />
            )}
        </>
    );
}

/** Draw your space in a browser, one tab, its Observability app open on the invite's issue. */
function SpaceDashboard() {
    return (
        <div data-component="Browser" {...style.attrs(browserStyles.browser, styles.browser)}>
            <div data-component="TabStrip" {...style.attrs(browserStyles.strip)}>
                <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                <span data-component="Tab" {...style.attrs(browserStyles.tab, browserStyles.tabOn)}>
                    <Favicon icon="user" tint={palette.teal} />
                    <span {...style.attrs(browserStyles.tabTitle)}>Florian</span>
                    <Glyph name="close" size={12} />
                </span>
            </div>
            <Address url="destack.app/@florian/observability/issues/obs-41" />
            <div {...style.attrs(browserStyles.viewport, styles.viewport)}>
                <div {...style.attrs(styles.scaled)}>
                    <ObservabilityIssue />
                </div>
            </div>
        </div>
    );
}

/** Draw the browser's address bar. */
function Address(properties: { url: string }) {
    const host = () => properties.url.split("/")[0] ?? "";

    return (
        <div data-component="Toolbar" {...style.attrs(browserStyles.toolbar)}>
            <span {...style.attrs(browserStyles.tools, browserStyles.desktopOnly)}>
                <Glyph name="back" />
                <Glyph name="forward" />
                <Glyph name="reload" />
            </span>
            <span data-component="AddressBar" {...style.attrs(browserStyles.omnibox)}>
                <Glyph name="lock" size={12} />
                <span {...style.attrs(browserStyles.url)}>
                    <span {...style.attrs(browserStyles.host)}>{host()}</span>
                    <span {...style.attrs(browserStyles.path)}>
                        {properties.url.slice(host().length)}
                    </span>
                </span>
            </span>
        </div>
    );
}

/** The inspector styles. */
const styles = style.create({
    fade: {
        display: "grid",
        height: "100%",
        minHeight: 0,
    },
    browser: {
        boxShadow: "none",
        height: "100%",
    },
    investigation: {
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: chrome.radius,
        borderStyle: "solid",
        borderWidth: stroke.border,
        bottom: "0.75rem",
        boxShadow: shadow.raised,
        display: "grid",
        fontSize: "0.75rem",
        gap: "1.25rem",
        gridTemplateColumns: {
            default: "minmax(0, 2fr) minmax(0, 1fr)",
            [screen.belowDesktop]: "1fr",
        },
        insetInline: "0.75rem",
        paddingBlock: "0.75rem",
        paddingInline: "1rem",
        position: "absolute",
        zIndex: 1,
    },
    found: {
        alignContent: "start",
        columnGap: "1rem",
        display: "grid",
        gridTemplateColumns: { default: "repeat(2, minmax(0, 1fr))", [screen.belowDesktop]: "1fr" },
        rowGap: "0.375rem",
    },
    investigationLabel: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.625rem",
        gridColumn: "1 / -1",
        letterSpacing: "0.1em",
        textTransform: "uppercase",
    },
    finding: {
        alignItems: "center",
        display: "flex",
        gap: "0.4375rem",
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    open: {
        alignContent: "start",
        borderInlineStartColor: color.border,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: { default: stroke.border, [screen.belowDesktop]: 0 },
        display: "grid",
        paddingInlineStart: { default: "1.25rem", [screen.belowDesktop]: 0 },
        rowGap: "0.375rem",
    },
    question: {
        alignItems: "center",
        display: "flex",
        fontWeight: 600,
        gap: "0.4375rem",
    },
    questionMark: {
        alignItems: "center",
        backgroundColor: `color-mix(in srgb, ${palette.ember} 14%, transparent)`,
        borderRadius: "50%",
        color: palette.ember,
        display: "flex",
        flexShrink: 0,
        fontSize: "0.6875rem",
        height: "1.125rem",
        justifyContent: "center",
        width: "1.125rem",
    },
    favicon: {
        borderRadius: "3px",
        flexShrink: 0,
        height: "14px",
        width: "14px",
    },
    viewport: {
        overflow: "hidden",
        position: "relative",
    },
    scaled: {
        containerType: "inline-size",
        height: "125%",
        left: 0,
        overflow: "clip",
        position: "absolute",
        top: 0,
        transform: "scale(0.8)",
        transformOrigin: "0 0",
        width: "125%",
    },
});
