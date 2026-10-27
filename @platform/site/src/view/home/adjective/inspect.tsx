import * as style from "@destack/style";
import { present } from "@destack/schema";
import { createSignal, type JSX } from "@destack/view";

import { palette } from "../../palette.stylex";
import { browserStyles } from "../figure/browser";
import { Fade } from "../figure/fade";
import { Favicon, Glyph } from "../figure/glyph";
import { AuditLog } from "./log";
import { AirtableSignupsOf } from "../tool/airtable";
import { GmailOf } from "../tool/gmail";
import { NotionPage } from "../tool/notion";
import { SlackChannel } from "../tool/slack";
import { TrelloBoard } from "../tool/trello";
import type { Mail, Message, Signup, Task } from "../verb/corner";
import type { Lane } from "./session";

/** A rented app you open to see what your agent did last night. */
export type Console = "gmail" | "airtable" | "slack" | "trello" | "notion";

/** The launch email's batches, newest first: Gmail sends to at most 500 recipients a message. */
const batches: readonly (readonly [recipients: number, time: string])[] = [
    [340, "17:24"],
    [500, "17:24"],
    [500, "17:23"],
    [500, "17:23"],
];

/** The sent folder: the launch email in its batches, then Ines's invite from the afternoon. */
const sent: readonly Mail[] = [
    ...batches.map(([recipients, time]): Mail => [
        `To: Waitlist (${recipients})`,
        "#ea4335",
        "Destack opens Friday",
        "Hi there, Destack opens on Friday, and you're in.",
        time,
        false,
    ]),
    [
        "Ines Ferreira",
        "#a0485f",
        "Your Destack invite",
        "Welcome in! Your invite link works from Friday 09:00.",
        "16:10",
        false,
    ],
];

/** The signups invited Thursday afternoon, each on the waitlist the email went to. */
const invited: readonly Signup[] = [
    ["ines@orbitlabs.eu", "Website", "Oct 18", "invited", "invited Thu 16:10"],
    ["jonas@kettle.dev", "Website", "Oct 19", "invited", "invited Thu 16:10"],
    ["priya@nair.studio", "Launch post", "Oct 19", "invited", "invited Thu 16:10"],
    ["maya@northwind.io", "Launch post", "Oct 20", "invited", "invited Thu 16:10"],
    ["tara@fieldnotes.co", "Friend", "Oct 20", "invited", "invited Thu 16:10"],
    ["sam@quietly.so", "Launch post", "Oct 21", "invited", "invited Thu 16:10"],
    ["leo@brightpath.app", "Website", "Oct 21", "invited", "invited Thu 16:10"],
];

/** The launch channel on launch morning: your questions, and no sign of a scheduled post. */
const posts: readonly Message[] = [
    ["Florian", "Thu 16:12", "400 invites are out 🎉"],
    ["Florian", "09:12", "Ines says she got the launch email twice?"],
    ["Florian", "09:14", "and the pricing link on the launch post is a 404"],
    ["Florian", "09:15", "where's the launch post? wasn't it scheduled for 9?"],
];

/** The launch board: the demo cards, and the demo recording closed. */
const board: readonly Task[] = [
    ["DST-12", "Record the demo", "done", "F", "#2f7d8c", "Thu"],
    ["DST-14", "Press kit", "review", "F", "#2f7d8c", "Thu"],
    ["DST-31", "Demo · Ines (Orbit Labs)", "todo", "F", "#2f7d8c", "Mon 11:00"],
    ["DST-32", "Demo · Jonas (Kettle)", "todo", "F", "#2f7d8c", "Mon 14:00"],
    ["DST-33", "Demo · Priya (Nair Studio)", "todo", "F", "#2f7d8c", "Tue 10:00"],
    ["DST-34", "Demo · Leo (Brightpath)", "todo", "F", "#2f7d8c", "Tue 11:00"],
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
        "gmail",
        "gmail",
        "Sent Mail - Gmail",
        "mail.google.com/mail/u/0/#sent",
        () => <GmailOf folder="Sent" mails={sent} />,
    ],
    [
        "airtable",
        "airtable",
        "Launch: Signups - Airtable",
        "airtable.com/appLaunch/tblSignups",
        () => <AirtableSignupsOf signups={invited} />,
    ],
    [
        "trello",
        "trello",
        "Launch | Trello",
        "trello.com/b/launch",
        () => <TrelloBoard tasks={board} />,
    ],
    ["notion", "notion", "Pricing | Notion", "notion.so/symbol/Pricing", () => <PricingDraft />],
];

/** The colours of the vendor icons drawn from the glyph set. */
const tints: Readonly<Record<string, string>> = {
    slack: "#4a154b",
    gmail: "#ea4335",
    airtable: "#18bfff",
    trello: "#0052cc",
    notion: "#191919",
};

/** Draw launch day: across rented consoles that each see a sliver, or on your own dashboard. */
export function Inspector(properties: {
    isOpen: boolean;
    console: Console;
    lane: Lane;
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
                    <AuditLog lane={properties.lane} onTouch={properties.onTouch} />
                </SpaceBrowser>
            ) : (
                <RentedConsoles console={properties.console} />
            )}
        </Fade>
    );
}

/** Draw the pricing page in the site's workspace, edited Thursday and never published. */
function PricingDraft() {
    return (
        <NotionPage
            crumbs={[
                ["🌐", "Destack site"],
                ["💳", "Pricing"],
            ]}
            tools="editor"
            icon="💳"
            title="Pricing"
            properties={[
                ["status", "Status", { kind: "status", text: "Draft", isDraft: true }],
                ["link", "Path", { kind: "text", text: "/pricing · not published" }],
                ["clock", "Edited", { kind: "text", text: "Thu 16:40" }],
                ["person", "Owner", { kind: "person", name: "Florian" }],
            ]}
        >
            <p>Early pricing holds for everyone on the waitlist.</p>
        </NotionPage>
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
            <Address url="destack.app/@florian/log" />
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
