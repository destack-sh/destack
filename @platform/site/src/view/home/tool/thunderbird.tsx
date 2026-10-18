import * as style from "@destack/style";

import { mails } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { thunderbird } from "./thunderbird.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    mail: ["M3 6h18v12H3z", "m3 7 9 6 9-6"],
    contacts: [
        "M5 3h14v18H5z",
        "M12 12a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z",
        "M8 17a4 4 0 0 1 8 0",
    ],
    calendar: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
    tasks: ["M5 5h14v14H5z", "m8.5 12 2.5 2.5 4.5-5"],
    chat: ["M5 18.5 6 15a7.5 7.5 0 1 1 3 3z"],
    gear: [
        "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "M4 12h2",
        "M18 12h2",
        "M12 4v2",
        "M12 18v2",
        "m6.3 6.3 1.4 1.4",
        "m16.3 16.3 1.4 1.4",
        "m6.3 17.7 1.4-1.4",
        "m16.3 7.7 1.4-1.4",
    ],
    fetch: ["M12 4v11", "m7 10 5 5 5-5", "M5 20h14"],
    compose: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    menu: ["M4 7h16", "M4 12h16", "M4 17h16"],
    inbox: ["M4 13h4.5l1.5 3h4l1.5-3H20", "M6 5h12l2 8v6H4v-6z"],
    draft: ["M5 4h10l4 4v12H5z", "m9 15 5-5 2 2-5 5H9z"],
    sent: ["M4 12 20 4l-4 16-4-7z", "m12 13 8-9"],
    archive: ["M4 5h16v4H4z", "M5 9v10h14V9", "M10 13h4"],
    trash: ["M5 7h14", "M9 7V4h6v3", "M7 7l1 13h8l1-13"],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
    caret: ["m7 10 5 5 5-5"],
    filter: ["M4 6h16", "M7 12h10", "M10 18h4"],
    star: ["m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"],
    warning: ["M12 3 2 20h20z", "M12 9v5", "M12 17h.01"],
} satisfies Record<string, readonly string[]>;

/** The spaces on the left bar, Mail chosen. */
const spaces: readonly (keyof typeof glyphs)[] = ["mail", "contacts", "calendar", "tasks", "chat"];

/** The account's folders: name, glyph and unread count, the inbox chosen. */
const folders: readonly (readonly [name: string, glyph: keyof typeof glyphs, count: string])[] = [
    ["Inbox", "inbox", "2"],
    ["Drafts", "draft", "1"],
    ["Sent", "sent", ""],
    ["Archives", "archive", ""],
    ["Trash", "trash", ""],
];

/** Draw the launch mail in the desktop client, its server refusing the waitlist send. */
export function Thunderbird() {
    return (
        <div data-component="ThunderbirdPage" {...style.attrs(styles.page)}>
            {/* spaces bar */}
            <nav {...style.attrs(styles.spaces)}>
                {spaces.map((space) => (
                    <span {...style.attrs(styles.space, space === "mail" && styles.spaceOn)}>
                        <Mark name={space} size={18} />
                    </span>
                ))}
                <span {...style.attrs(styles.spacesEnd)}>
                    <Mark name="gear" size={18} />
                </span>
            </nav>

            <div {...style.attrs(styles.window)}>
                {/* unified toolbar */}
                <header {...style.attrs(styles.toolbar)}>
                    <span {...style.attrs(styles.tool, styles.wide)}>
                        <Mark name="fetch" size={16} />
                        Get Messages
                    </span>
                    <span {...style.attrs(styles.write)}>
                        <Mark name="compose" size={16} />
                        Write
                    </span>
                    <span {...style.attrs(styles.search)}>
                        <Mark name="search" size={14} />
                        Search…
                    </span>
                    <Mark name="menu" size={18} />
                </header>

                <div {...style.attrs(styles.panes)}>
                    {/* folders */}
                    <nav {...style.attrs(styles.folders)}>
                        <span {...style.attrs(styles.foldersHead)}>
                            <span {...style.attrs(styles.newMessage)}>
                                <Mark name="compose" size={14} />
                                New Message
                            </span>
                            <Mark name="more" size={16} />
                        </span>
                        <span {...style.attrs(styles.account)}>
                            <Mark name="caret" size={12} />
                            <Favicon icon="thunderbird" tint="#0a84ff" size={16} />
                            florian@symbol.co
                        </span>
                        {folders.map(([name, glyph, count], index) => (
                            <span {...style.attrs(styles.folder, index === 0 && styles.folderOn)}>
                                <Mark name={glyph} size={16} />
                                <span {...style.attrs(count !== "" && styles.folderUnread)}>
                                    {name}
                                </span>
                                {count === "" ? undefined : (
                                    <span {...style.attrs(styles.badge)}>{count}</span>
                                )}
                            </span>
                        ))}
                    </nav>

                    {/* message cards, the server's refusal over them */}
                    <section {...style.attrs(styles.list)}>
                        <span {...style.attrs(styles.listHead)}>
                            <b>Inbox</b>
                            <span {...style.attrs(styles.listCount)}>6 Messages</span>
                            <span {...style.attrs(styles.quickFilter)}>
                                <Mark name="filter" size={14} />
                                Quick Filter
                            </span>
                        </span>
                        {mails.map(([sender, , subject, , time, isUnread], index) => (
                            <span {...style.attrs(styles.card, index === 0 && styles.cardOn)}>
                                <span {...style.attrs(styles.cardTop)}>
                                    {isUnread ? (
                                        <span {...style.attrs(styles.unreadDot)} />
                                    ) : undefined}
                                    <span {...style.attrs(styles.sender, isUnread && styles.bold)}>
                                        {sender}
                                    </span>
                                    <span {...style.attrs(styles.time)}>{time}</span>
                                </span>
                                <span {...style.attrs(styles.cardBottom)}>
                                    <span {...style.attrs(styles.subject, isUnread && styles.bold)}>
                                        {subject}
                                    </span>
                                    <Mark name="star" size={14} />
                                </span>
                            </span>
                        ))}

                        <section role="alertdialog" {...style.attrs(styles.dialog)}>
                            <span {...style.attrs(styles.dialogMark)}>
                                <Mark name="warning" size={28} />
                            </span>
                            <span {...style.attrs(styles.dialogText)}>
                                <b>Send Message Error</b>
                                <span>
                                    Sending of the message failed. The mail server smtp.gmail.com
                                    responded: 452 4.5.3 Your message has too many recipients.
                                </span>
                                <span {...style.attrs(styles.dialogMeta)}>
                                    1,840 recipients. Gmail accepts 100 a message over SMTP.
                                </span>
                            </span>
                            <span {...style.attrs(styles.ok)}>OK</span>
                        </section>
                    </section>
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

/** The Thunderbird page styles. */
const styles = style.create({
    page: {
        backgroundColor: thunderbird.chrome,
        color: thunderbird.ink,
        display: "grid",
        fontFamily: thunderbird.face,
        fontSize: "13px",
        gridTemplateColumns: { default: "44px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
    },
    mark: {
        fill: "none",
        flexShrink: 0,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.6,
    },

    // spaces bar
    spaces: {
        alignItems: "center",
        color: thunderbird.muted,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "6px",
        paddingBlock: "10px",
    },
    space: {
        alignItems: "center",
        borderRadius: "4px",
        display: "inline-flex",
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },
    spaceOn: {
        backgroundColor: thunderbird.selected,
        color: thunderbird.blue,
    },
    spacesEnd: {
        marginBlockStart: "auto",
    },
    window: {
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: 0,
    },

    // unified toolbar
    toolbar: {
        alignItems: "center",
        color: thunderbird.ink,
        display: "flex",
        gap: "8px",
        height: "42px",
        paddingInline: "8px",
    },
    tool: {
        alignItems: "center",
        borderRadius: "4px",
        gap: "6px",
        paddingBlock: "5px",
        paddingInline: "8px",
    },
    wide: {
        display: { default: "inline-flex", [site.narrow]: "none" },
    },
    write: {
        alignItems: "center",
        borderRadius: "4px",
        display: "inline-flex",
        gap: "6px",
        paddingBlock: "5px",
        paddingInline: "8px",
    },
    search: {
        alignItems: "center",
        backgroundColor: thunderbird.field,
        borderColor: thunderbird.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: thunderbird.muted,
        display: "flex",
        flexGrow: 1,
        gap: "6px",
        height: "28px",
        marginInline: "8px",
        paddingInline: "8px",
    },
    panes: {
        display: "grid",
        gridTemplateColumns: { default: "210px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },

    // folders
    folders: {
        backgroundColor: thunderbird.folders,
        borderStartStartRadius: "6px",
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "1px",
        paddingInline: "6px",
    },
    foldersHead: {
        alignItems: "center",
        color: thunderbird.muted,
        display: "flex",
        gap: "6px",
        paddingBlock: "8px",
    },
    newMessage: {
        alignItems: "center",
        backgroundColor: thunderbird.blue,
        borderRadius: "4px",
        color: thunderbird.onAccent,
        display: "inline-flex",
        flexGrow: 1,
        fontWeight: 600,
        gap: "6px",
        justifyContent: "center",
        paddingBlock: "6px",
    },
    account: {
        alignItems: "center",
        display: "flex",
        fontWeight: 600,
        gap: "6px",
        paddingBlock: "6px",
        paddingInline: "2px",
    },
    folder: {
        alignItems: "center",
        borderRadius: "4px",
        color: thunderbird.ink,
        display: "flex",
        gap: "8px",
        paddingBlock: "5px",
        paddingInlineEnd: "6px",
        paddingInlineStart: "22px",
    },
    folderOn: {
        backgroundColor: thunderbird.selected,
    },
    folderUnread: {
        fontWeight: 700,
    },
    badge: {
        backgroundColor: thunderbird.blue,
        borderRadius: "999px",
        color: thunderbird.onAccent,
        fontSize: "11px",
        fontWeight: 700,
        marginInlineStart: "auto",
        paddingInline: "6px",
    },

    // message cards
    list: {
        backgroundColor: thunderbird.surface,
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        position: "relative",
    },
    listHead: {
        alignItems: "center",
        borderBottomColor: thunderbird.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "8px",
        paddingBlock: "8px",
        paddingInline: "12px",
    },
    listCount: {
        color: thunderbird.muted,
    },
    quickFilter: {
        alignItems: "center",
        color: thunderbird.muted,
        display: "flex",
        gap: "4px",
        marginInlineStart: "auto",
    },
    card: {
        borderBottomColor: thunderbird.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "grid",
        gap: "3px",
        paddingBlock: "7px",
        paddingInline: "14px",
    },
    cardOn: {
        backgroundColor: thunderbird.selected,
    },
    cardTop: {
        alignItems: "center",
        display: "flex",
        gap: "6px",
    },
    unreadDot: {
        backgroundColor: thunderbird.unread,
        borderRadius: "50%",
        height: "7px",
        width: "7px",
    },
    sender: {
        flexGrow: 1,
        overflow: "hidden",
        whiteSpace: "nowrap",
    },
    bold: {
        fontWeight: 700,
    },
    time: {
        color: thunderbird.muted,
        fontSize: "12px",
    },
    cardBottom: {
        alignItems: "center",
        color: thunderbird.muted,
        display: "flex",
        gap: "6px",
    },
    subject: {
        color: thunderbird.ink,
        flexGrow: 1,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },

    // error dialog
    dialog: {
        alignItems: "start",
        backgroundColor: thunderbird.surface,
        borderRadius: "8px",
        boxShadow: thunderbird.lift,
        columnGap: "14px",
        display: "grid",
        gridTemplateColumns: "auto minmax(0, 1fr)",
        insetInline: "8%",
        lineHeight: 1.45,
        padding: "18px",
        position: "absolute",
        rowGap: "14px",
        top: "30%",
    },
    dialogMark: {
        color: thunderbird.error,
    },
    dialogText: {
        display: "grid",
        gap: "6px",
    },
    dialogMeta: {
        color: thunderbird.muted,
        fontSize: "12px",
    },
    ok: {
        backgroundColor: thunderbird.blue,
        borderRadius: "4px",
        color: thunderbird.onAccent,
        fontWeight: 600,
        gridColumn: "1 / -1",
        justifySelf: "end",
        paddingBlock: "5px",
        paddingInline: "22px",
    },
});
