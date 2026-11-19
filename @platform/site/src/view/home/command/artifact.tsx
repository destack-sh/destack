import * as style from "@destack/style";
import type { JSX } from "@destack/view";

import { Favicon } from "../figure/glyph";
import { type Assistant, themes } from "./chat";

/** The assistants that publish an app they made as an artifact at a public link. */
export const publishers = [
    "Claude",
    "ChatGPT",
    "Gemini",
    "Grok",
] as const satisfies readonly Assistant[];

/** An assistant that publishes an app it made as an artifact. */
export type Publisher = (typeof publishers)[number];
import { chat } from "./chat.stylex";

/** How each assistant publishes an app it made: its mark, address, header actions and notice. */
const publications: Readonly<
    Record<
        Publisher,
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

/** Return the address an assistant publishes an app at, split into its host and path. */
export function artifactAddressOf(assistant: Publisher): { host: string; path: string } {
    const site = publications[assistant].site;
    const slash = site.indexOf("/");

    return { host: site.slice(0, slash), path: site.slice(slash) };
}

/** Return the mark an assistant's published apps carry in a browser tab. */
export function artifactMarkOf(assistant: Publisher): { icon: string; tint: string } {
    return { icon: publications[assistant].icon, tint: publications[assistant].tint };
}

/** Draw an app an assistant published, in its page: header, sandbox and notice. */
export function ArtifactFrame(properties: {
    assistant: Publisher;
    title: string;
    children: JSX.Element;
}) {
    const publisher = () => publications[properties.assistant];

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
            <main {...style.attrs(styles.frame)}>{properties.children}</main>

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
        backgroundColor: chat.field,
        margin: "16px",
        borderColor: chat.border,
        borderRadius: "12px",
        borderStyle: "solid",
        borderWidth: "1px",
        overflow: "hidden",
    },
    notice: {
        color: chat.muted,
        fontSize: "11px",
        paddingBlockEnd: "12px",
        textAlign: "center",
    },
});
