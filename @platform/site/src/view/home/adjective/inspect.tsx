import * as style from "@destack/style";
import { createSignal, type JSX } from "@destack/view";

import { palette } from "../../palette.stylex";
import { browserStyles } from "../figure/browser";
import { Fade } from "../figure/fade";
import { Favicon, Glyph } from "../figure/glyph";
import { LaunchDashboard } from "./dashboard";
import { ResendLogs, SentryIssue, SupabaseQuery, TemporalRun } from "./console";
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
    ["resend", "resend.svg", "Logs · Resend", "resend.com/logs", ResendLogs],
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

/** Draw the rented consoles in a browser, a tab each, as each vendor draws its page. */
function RentedConsoles() {
    // hold the console open
    const [open, setOpen] = createSignal<Console>("sentry");
    const shown = () => consoles.find(([key]) => key === open()) ?? consoles[0];

    return (
        <div data-component="Browser" {...style.attrs(browserStyles.browser, styles.browser)}>
            <div data-component="TabStrip" {...style.attrs(browserStyles.strip)}>
                <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                {consoles.map(([key, logo, title]) => (
                    <span
                        data-component="Tab"
                        title={title}
                        onClick={() => setOpen(key)}
                        {...style.attrs(
                            browserStyles.tab,
                            browserStyles.tabEven,
                            key === open() && browserStyles.tabOn,
                            browserStyles.tabSteady,
                        )}
                    >
                        <TabMark logo={logo} />
                        <span {...style.attrs(browserStyles.tabTitle)}>{title}</span>
                    </span>
                ))}
            </div>
            <Address url={shown()?.[3] ?? ""} />
            <div {...style.attrs(browserStyles.viewport, styles.viewport)}>
                <div {...style.attrs(styles.scaled)}>{shown()?.[4]()}</div>
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

/** Draw your space in a browser, one tab, its Observability app open on the launch dashboard. */
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
            <Address url="destack.app/@florian/observability/dashboards/launch" />
            <div {...style.attrs(browserStyles.viewport, styles.viewport)}>
                <div {...style.attrs(styles.scaled)}>
                    <LaunchDashboard />
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
