import * as style from "@destack/style";

import { Favicon } from "../figure/glyph";
import type { Vendor } from "../verb/space";
import { type Assistant, themes } from "./chat";
import { chat } from "./chat.stylex";
import type { Lead } from "./scene";

/** How each assistant publishes an app it made: its mark, address, header actions and notice. */
const publishers: Readonly<
    Record<
        Assistant,
        {
            icon: string;
            tint: string;
            site: string;
            secondary: string;
            primary: string;
            notice: string;
        }
    >
> = {
    Claude: {
        icon: "claude",
        tint: "#c96442",
        site: "claude.ai/public/artifacts/7f3c2a91-4e0b",
        secondary: "Copy",
        primary: "Customize",
        notice: "Content is user-generated and unverified.",
    },
    ChatGPT: {
        icon: "openai",
        tint: "#0d0d0d",
        site: "chatgpt.com/canvas/shared/68f2c1d4a0b8",
        secondary: "Log in",
        primary: "Sign up",
        notice: "This is a copy of a canvas shared by Florian.",
    },
    Gemini: {
        icon: "gemini",
        tint: "#4285f4",
        site: "gemini.google.com/share/a41c9e2b7d03",
        secondary: "Copy",
        primary: "Try Gemini",
        notice: "Shared canvas · Gemini can make mistakes.",
    },
    Grok: {
        icon: "grok",
        tint: "#0d0d0d",
        site: "grok.com/share/c/5d8e40f1",
        secondary: "Copy",
        primary: "Try Grok",
        notice: "Shared from Grok.",
    },
};

/** Return the published app an assistant made, as the site the browser opens on a row. */
export function artifactOf(
    assistant: Assistant,
    row: number,
    title: string,
    rows: readonly Lead[],
): Vendor {
    const publisher = publishers[assistant];

    return {
        row,
        icon: publisher.icon,
        tint: publisher.tint,
        tab: title,
        site: publisher.site,
        page: () => <ArtifactPage assistant={assistant} title={title} rows={rows} />,
    };
}

/** Draw an app an assistant made at its public link, in that assistant's own page. */
function ArtifactPage(properties: { assistant: Assistant; title: string; rows: readonly Lead[] }) {
    const publisher = () => publishers[properties.assistant];

    return (
        <div
            data-component="ArtifactPage"
            data-assistant={properties.assistant}
            {...style.attrs(themes[properties.assistant], styles.page)}
        >
            {/* the assistant's header: its mark, the app's title, and its actions */}
            <header {...style.attrs(styles.header)}>
                <span {...style.attrs(styles.brand)}>
                    <Favicon icon={publisher().icon} tint={publisher().tint} size={18} />
                    {properties.assistant}
                </span>
                <span {...style.attrs(styles.title)}>{properties.title}</span>
                <span {...style.attrs(styles.actions)}>
                    <span {...style.attrs(styles.secondary)}>{publisher().secondary}</span>
                    <span {...style.attrs(styles.primary)}>{publisher().primary}</span>
                </span>
            </header>

            {/* the app itself, sandboxed in the page */}
            <main {...style.attrs(styles.frame)}>
                <b {...style.attrs(styles.heading)}>{properties.title}</b>
                <span {...style.attrs(styles.connect)}>
                    Connect your own Airtable and Slack to load this board
                </span>
                <span {...style.attrs(styles.table)}>
                    {properties.rows.map(([name, asked, state]) => (
                        <span {...style.attrs(styles.row)}>
                            <span {...style.attrs(styles.name)}>{name}</span>
                            <span {...style.attrs(styles.asked)}>{asked}</span>
                            <span {...style.attrs(styles.state)}>{state}</span>
                        </span>
                    ))}
                </span>
            </main>

            {/* the assistant's notice under every public app */}
            <footer {...style.attrs(styles.notice)}>{publisher().notice}</footer>
        </div>
    );
}

/** The published artifact page styles. */
const styles = style.create({
    page: {
        backgroundColor: chat.canvas,
        color: chat.ink,
        display: "grid",
        fontFamily: 'ui-sans-serif, -apple-system, "Segoe UI", system-ui, sans-serif',
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        height: "100%",
        overflow: "hidden",
    },
    header: {
        alignItems: "center",
        borderBottomColor: chat.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "grid",
        gap: "12px",
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
        paddingBlock: "10px",
        paddingInline: "16px",
    },
    brand: {
        alignItems: "center",
        display: "inline-flex",
        fontSize: "15px",
        fontWeight: 600,
        gap: "8px",
    },
    title: {
        color: chat.muted,
        fontSize: "13px",
        overflow: "hidden",
        textAlign: "center",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    actions: {
        alignItems: "center",
        display: "inline-flex",
        fontSize: "13px",
        gap: "8px",
    },
    secondary: {
        borderColor: chat.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        paddingBlock: "4px",
        paddingInline: "10px",
    },
    primary: {
        backgroundColor: chat.ink,
        borderRadius: "8px",
        color: chat.canvas,
        fontWeight: 500,
        paddingBlock: "5px",
        paddingInline: "11px",
    },
    frame: {
        alignContent: "start",
        backgroundColor: chat.field,
        display: "grid",
        gap: "10px",
        margin: "16px",
        borderColor: chat.border,
        borderRadius: "12px",
        borderStyle: "solid",
        borderWidth: "1px",
        overflow: "hidden",
        padding: "20px",
    },
    heading: {
        fontFamily: chat.replyFont,
        fontSize: "20px",
        fontWeight: 600,
    },
    connect: {
        color: chat.muted,
        fontSize: "12px",
    },
    table: {
        backgroundColor: chat.canvas,
        borderColor: chat.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        fontSize: "13px",
    },
    row: {
        alignItems: "center",
        borderBottomColor: chat.border,
        borderBottomStyle: { default: "solid", ":last-child": "none" },
        borderBottomWidth: "1px",
        display: "grid",
        gap: "10px",
        gridTemplateColumns: "minmax(0, 7.5rem) minmax(0, 1fr) auto",
        paddingBlock: "8px",
        paddingInline: "12px",
    },
    name: {
        fontWeight: 500,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    asked: {
        color: chat.muted,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    state: {
        fontSize: "12px",
        whiteSpace: "nowrap",
    },
    notice: {
        color: chat.muted,
        fontSize: "11px",
        paddingBlockEnd: "12px",
        textAlign: "center",
    },
});
