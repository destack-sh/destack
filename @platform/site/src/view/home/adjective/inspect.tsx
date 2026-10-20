import * as style from "@destack/style";
import { present } from "@destack/schema";
import { createSignal, type JSX } from "@destack/view";

import { palette } from "../../palette.stylex";
import { browserStyles } from "../figure/browser";
import { Fade } from "../figure/fade";
import { Favicon, Glyph } from "../figure/glyph";
import { type RunMoment, RunScreen } from "./timeline";
import { AirtableSignupsOf } from "../tool/airtable";
import { GoogleCalendarWeek } from "../tool/googlecalendar";
import { SlackChannel } from "../tool/slack";
import { TrelloBoard } from "../tool/trello";
import { ZapierHistory, type ZapRun } from "../tool/zapier";
import type { Message, Signup, Task, WeekBlock } from "../verb/corner";
import { askers } from "./trace";

/** A rented app you open to see what the Zap did. */
export type Console = "zapier" | "airtable" | "trello" | "slack" | "googlecalendar";

/** The Zap's nine runs after the approval, newest first. */
const zapRuns: readonly ZapRun[] = askers
    .map(([name, company], index): ZapRun => [
        `${name} (${company})`,
        `Oct 23, 2025 09:00:0${Math.floor((44 + index * 135) / 1000)} am`,
    ])
    .toReversed();

/** The signups just after the bulk approve: every row modified at 09:00, demo askers among them. */
const approved: readonly Signup[] = [
    ["ines@orbitlabs.eu", "Website", "Oct 18", "approved", "demo asker · edited 09:00"],
    ["jonas@kettle.dev", "Website", "Oct 19", "approved", "demo asker · edited 09:00"],
    ["priya@nair.studio", "Launch post", "Oct 19", "approved", "demo asker · edited 09:00"],
    ["maya@northwind.io", "Launch post", "Oct 20", "approved", "edited 09:00 · Florian"],
    ["tara@fieldnotes.co", "Friend", "Oct 20", "approved", "edited 09:00 · Florian"],
    ["sam@quietly.so", "Launch post", "Oct 21", "approved", "edited 09:00 · Florian"],
    ["leo@brightpath.app", "Website", "Oct 21", "approved", "demo asker · edited 09:00"],
];

/** The launch channel: the posts from Monday, then the same posts again at 09:00. */
const posts: readonly Message[] = [
    ["Zapier", "Mon 10:14", "New demo request: Ines · Orbit Labs · Wed 11:00"],
    ["Zapier", "Mon 11:02", "New demo request: Jonas · Kettle · Thu 14:00"],
    ["Florian", "Mon 11:05", "Nice, two already."],
    ["Zapier", "09:00", "New demo request: Ines · Orbit Labs · Wed 11:00"],
    ["Zapier", "09:00", "New demo request: Jonas · Kettle · Thu 14:00"],
    ["Zapier", "09:00", "New demo request: Priya · Nair Studio · Thu 15:00"],
    ["Florian", "09:02", "why is everyone asking twice?"],
];

/** The launch board: each asker's demo card, and its twin from 09:00. */
const demoCards: readonly Task[] = askers
    .slice(0, 4)
    .flatMap(([name, company, task, demo]): Task[] => [
        [task, `Demo · ${name} (${company})`, "todo", "F", "#2f7d8c", demo],
        [`${task}b`, `Demo · ${name} (${company})`, "todo", "Z", "#ff4f00", demo],
    ]);

/** The week: the demos booked, and Ines's booked again at the same time. */
const demoWeek: readonly WeekBlock[] = [
    [0, 9, 9.5, "Standup", false],
    [2, 11, 12, "Demo with Ines · Destack", false],
    [2, 11, 12, "Demo with Ines · Destack", false],
    [3, 9, 10, "Approve signups", false],
    [3, 14, 15, "Demo with Jonas · Destack", false],
    [3, 15, 16, "Demo with Priya · Destack", false],
    [4, 10, 11, "Demo with Leo · Destack", false],
    [4, 11, 12, "Demo with Noor · Destack", false],
];

/** The apps you rent, as tabs: key, logo, title, address and page. */
const consoles: readonly (readonly [
    key: Console,
    logo: string,
    title: string,
    url: string,
    page: () => JSX.Element,
])[] = [
    [
        "slack",
        "slack",
        "launch (Channel) - Symbol - Slack",
        "app.slack.com/client/symbol/launch",
        () => <SlackChannel messages={posts} />,
    ],
    [
        "zapier",
        "zapier",
        "Zap history | Zapier",
        "zapier.com/app/history",
        () => <ZapierHistory zap="Launch askers" runs={zapRuns} />,
    ],
    [
        "airtable",
        "airtable",
        "Launch: Signups - Airtable",
        "airtable.com/appLaunch/tblSignups",
        () => <AirtableSignupsOf signups={approved} />,
    ],
    [
        "trello",
        "trello",
        "Launch | Trello",
        "trello.com/b/launch",
        () => <TrelloBoard tasks={demoCards} />,
    ],
    [
        "googlecalendar",
        "googlecalendar",
        "Google Calendar - Week of October 20",
        "calendar.google.com/calendar/r/week",
        () => <GoogleCalendarWeek week={demoWeek} />,
    ],
];

/** The colours of the vendor icons drawn from the glyph set. */
const tints: Readonly<Record<string, string>> = {
    slack: "#4a154b",
    zapier: "#ff4f00",
    airtable: "#18bfff",
    trello: "#0052cc",
    googlecalendar: "#4285f4",
};

/** Draw launch day: across rented consoles that each see a sliver, or on your own dashboard. */
export function Inspector(properties: {
    isOpen: boolean;
    console: Console;
    moment: RunMoment;
    onScrub: (share: number) => void;
    onTouch: () => void;
}) {
    return (
        <Fade
            isOpen={properties.isOpen}
            xstyle={styles.fade}
            isLarge
            component={properties.isOpen ? "SpaceDashboard" : "RentedConsoles"}
        >
            {properties.isOpen ? (
                <SpaceBrowser>
                    <RunScreen
                        moment={properties.moment}
                        onScrub={properties.onScrub}
                        onTouch={properties.onTouch}
                    />
                </SpaceBrowser>
            ) : (
                <RentedConsoles console={properties.console} />
            )}
        </Fade>
    );
}

/** Draw the rented consoles in a browser, a tab each turning on its own, and what each told you. */
function RentedConsoles(properties: { console: Console }) {
    // show the console your agent reads, or the one the reader opens
    const [opened, setOpened] = createSignal<Console>();
    const index = () => consoles.findIndex(([key]) => key === (opened() ?? properties.console));
    const shown = () => present(consoles[index()], "shown console");

    return (
        <div
            data-component="Browser"
            data-morph="inspect"
            {...style.attrs(browserStyles.browser, styles.browser)}
        >
            <div data-component="TabStrip" data-morph-strip {...style.attrs(browserStyles.strip)}>
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
            <div data-morph-page {...style.attrs(browserStyles.viewport, styles.viewport)}>
                <div {...style.attrs(styles.scaled)}>{shown()[4]()}</div>
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

/** Draw your space in a browser, one tab, on the approval's transaction. */
function SpaceBrowser(properties: { children: JSX.Element }) {
    return (
        <div
            data-component="Browser"
            data-morph="inspect"
            {...style.attrs(browserStyles.browser, styles.browser)}
        >
            <div data-component="TabStrip" data-morph-strip {...style.attrs(browserStyles.strip)}>
                <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                <span data-component="Tab" {...style.attrs(browserStyles.tab, browserStyles.tabOn)}>
                    <Favicon icon="user" tint={palette.teal} />
                    <span {...style.attrs(browserStyles.tabTitle)}>Florian</span>
                    <Glyph name="close" size={12} />
                </span>
            </div>
            <Address url="destack.app/@florian/observability/transactions/tx-91c" />
            <div data-morph-page {...style.attrs(browserStyles.viewport, styles.viewport)}>
                <div {...style.attrs(styles.full)}>{properties.children}</div>
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
    full: {
        containerType: "inline-size",
        inset: 0,
        overflow: "clip",
        position: "absolute",
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
