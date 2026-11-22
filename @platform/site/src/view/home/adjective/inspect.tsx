import * as style from "@destack/style";
import { present } from "@destack/schema";
import { createSignal, type JSX } from "@destack/view";

import { palette } from "../../palette.stylex";
import { browserStyles } from "../figure/browser";
import { Fade } from "../figure/fade";
import { Favicon, Glyph } from "../figure/glyph";
import { AuditLog } from "./log";
import { GmailOf } from "../tool/gmail";
import { type Mail, type Story, useStory } from "../verb/story";
import type { Lane } from "./session";
import { Cast } from "../figure/cast";

/** A place in your rented mail you open to see what your agent did last night. */
export type Console = "gmail" | "receipts" | "archive";

/** Read the inbox after the night's filters: the mail neither filed as a receipt nor archived. */
function inboxOf(story: Story): readonly Mail[] {
    return story.mails.filter(
        ([sender, , subject]) =>
            !story.receipts.some((mail) => mail[0] === sender) &&
            !story.archived.some((mail) => mail[2] === subject),
    );
}

/** The places your rented mail shows the night's work: key, logo, title, address and page. */
const consoles: readonly (readonly [
    key: Console,
    logo: string,
    title: string,
    url: string,
    page: (story: Story) => JSX.Element,
])[] = [
    [
        "gmail",
        "gmail",
        "Inbox - Gmail",
        "mail.google.com/mail/u/0/#inbox",
        (story) => <GmailOf folder="Inbox" mails={inboxOf(story)} />,
    ],
    [
        "receipts",
        "gmail",
        "Receipts - Gmail",
        "mail.google.com/mail/u/0/#label/Receipts",
        (story) => <GmailOf folder="Receipts" mails={story.receipts} />,
    ],
    [
        "archive",
        "gmail",
        "All Mail - Gmail",
        "mail.google.com/mail/u/0/#all",
        (story) => <GmailOf folder="All Mail" mails={story.archived} />,
    ],
];

/** The colours of the vendor icons drawn from the glyph set. */
const tints: Readonly<Record<string, string>> = {
    gmail: "#ea4335",
};

/** Draw last night's work: across your rented mail's folders, or in your space's log. */
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

/** Draw the rented consoles in a browser, a tab each turning on its own, and what each told you. */
function RentedConsoles(properties: { console: Console }) {
    // show the console your agent reads, or the one the reader opens, in the page's story
    const story = useStory();
    const [opened, setOpened] = createSignal<Console>();
    const index = () => consoles.findIndex(([key]) => key === (opened() ?? properties.console));
    const shown = () => present(consoles[index()], "shown console");

    return (
        <Cast>
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
                    <div {...style.attrs(styles.scaled)}>{shown()[4](story())}</div>
                </div>
            </div>
        </Cast>
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
        <Cast>
            <div data-component="Browser" {...style.attrs(browserStyles.browser, styles.browser)}>
                <div data-component="TabStrip" {...style.attrs(browserStyles.strip)}>
                    <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                    <span
                        data-component="Tab"
                        {...style.attrs(browserStyles.tab, browserStyles.tabOn)}
                    >
                        <Favicon icon="user" tint={palette.teal} />
                        <span {...style.attrs(browserStyles.tabTitle)}>Florian</span>
                        <Glyph name="close" size={12} />
                    </span>
                </div>
                <Address url="destack.app/@florian/log" />
                <div {...style.attrs(browserStyles.viewport, styles.viewport)}>
                    <div {...style.attrs(styles.full)}>{properties.children}</div>
                </div>
            </div>
        </Cast>
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
