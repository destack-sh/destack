import * as style from "@destack/style";

import { mails } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { outlook } from "./outlook.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    launcher: [
        "M5 5h.01",
        "M12 5h.01",
        "M19 5h.01",
        "M5 12h.01",
        "M12 12h.01",
        "M19 12h.01",
        "M5 19h.01",
        "M12 19h.01",
        "M19 19h.01",
    ],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    mail: ["M3 6h18v12H3z", "m3 7 9 6 9-6"],
    calendar: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
    people: [
        "M9 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z",
        "M3 20a6 6 0 0 1 12 0",
        "M16 11a3 3 0 1 0 0-6",
        "M17.5 14.5A5 5 0 0 1 21 19.5",
    ],
    files: ["M7 3h7l5 5v13H7z", "M14 3v5h5"],
    todo: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "m8 12 3 3 5-6"],
    compose: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    trash: ["M5 7h14", "M9 7V4h6v3", "M7 7l1 13h8l1-13"],
    archive: ["M4 5h16v4H4z", "M5 9v10h14V9", "M10 13h4"],
    reply: ["M10 8 5 12l5 4", "M5 12h9a5 5 0 0 1 5 5v1"],
    flag: ["M6 21V4", "M6 4h11l-2 4 2 4H6"],
    caret: ["m7 10 5 5 5-5"],
    right: ["m10 7 5 5-5 5"],
    inbox: ["M4 13h4.5l1.5 3h4l1.5-3H20", "M6 5h12l2 8v6H4v-6z"],
    draft: ["M5 4h10l4 4v12H5z", "m9 15 5-5 2 2-5 5H9z"],
    sent: ["M4 12 20 4l-4 16-4-7z", "m12 13 8-9"],
    filter: ["M4 6h16", "M7 12h10", "M10 18h4"],
    attach: ["m20 11-8.5 8.5a5 5 0 0 1-7-7L13 4a3.5 3.5 0 0 1 5 5l-8.5 8.5a2 2 0 0 1-3-3L14 7"],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
    bell: ["M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z", "M10 20.5a2 2 0 0 0 4 0"],
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
    warning: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 7v6", "M12 16.5h.01"],
    shield: ["M12 3 5 6v6c0 4.5 3 7.5 7 9 4-1.5 7-4.5 7-9V6z", "M12 8v5", "M12 16h.01"],
    sweep: ["M14 4 9 13", "M5 14h8l-1 7H6z", "m15 9 5-3"],
    replyAll: ["M11 8 6 12l5 4", "M7 8l-4 4 4 4", "M6 12h8a5 5 0 0 1 5 5v1"],
    forward: ["M14 8l5 4-5 4", "M19 12h-9a5 5 0 0 0-5 5v1"],
    bolt: ["M13 3 5 14h6l-1 7 8-11h-6z"],
    read: ["M3 9l9-6 9 6v11H3z", "m3 9 9 6 9-6"],
} satisfies Record<string, readonly string[]>;

/** The rail's apps, Mail chosen. */
const apps: readonly (keyof typeof glyphs)[] = ["mail", "calendar", "people", "files", "todo"];

/** The account's folders: name, glyph and unread count, the inbox chosen. */
const folders: readonly (readonly [name: string, glyph: keyof typeof glyphs, count: string])[] = [
    ["Inbox", "inbox", "2"],
    ["Drafts", "draft", "1"],
    ["Sent Items", "sent", ""],
    ["Archive", "archive", ""],
    ["Deleted Items", "trash", ""],
];

/** Draw the launch mail in the office inbox, the waitlist pasted into a new message. */
export function Outlook() {
    return (
        <div data-component="OutlookPage" {...style.attrs(styles.page)}>
            {/* header bar */}
            <header {...style.attrs(styles.header)}>
                <span {...style.attrs(styles.brand)}>
                    <Favicon icon="outlook" tint="#0f6cbd" size={18} />
                    Outlook
                </span>
                <span {...style.attrs(styles.search)}>
                    <Mark name="search" size={16} />
                    Search
                </span>
                <span {...style.attrs(styles.headerEnd)}>
                    <Mark name="bell" size={18} />
                    <Mark name="gear" size={18} />
                    <span {...style.attrs(styles.me)}>F</span>
                </span>
            </header>

            <div {...style.attrs(styles.body)}>
                {/* app rail */}
                <nav {...style.attrs(styles.rail)}>
                    {apps.map((app) => (
                        <span {...style.attrs(styles.app, app === "mail" && styles.appOn)}>
                            <Mark name={app} size={20} />
                        </span>
                    ))}
                </nav>

                <div {...style.attrs(styles.work)}>
                    {/* simplified ribbon */}
                    <div {...style.attrs(styles.ribbon)}>
                        <span {...style.attrs(styles.ribbonTabs)}>
                            <span {...style.attrs(styles.ribbonTabOn)}>Home</span>
                            <span>View</span>
                            <span>Help</span>
                        </span>
                        <span {...style.attrs(styles.commands)}>
                            <span {...style.attrs(styles.newMail)}>
                                <Mark name="compose" size={16} />
                                New mail
                                <span {...style.attrs(styles.newMailCaret)}>
                                    <Mark name="caret" size={12} />
                                </span>
                            </span>
                            <span {...style.attrs(styles.command, styles.iconOnly)}>
                                <Mark name="trash" size={17} />
                            </span>
                            <span {...style.attrs(styles.command, styles.iconOnly, styles.green)}>
                                <Mark name="archive" size={17} />
                            </span>
                            <span {...style.attrs(styles.command, styles.iconOnly, styles.red)}>
                                <Mark name="shield" size={17} />
                            </span>
                            <span {...style.attrs(styles.command, styles.iconOnly)}>
                                <Mark name="sweep" size={17} />
                            </span>
                            <span {...style.attrs(styles.divider)} />
                            <span {...style.attrs(styles.command, styles.iconOnly, styles.purple)}>
                                <Mark name="reply" size={17} />
                            </span>
                            <span {...style.attrs(styles.command, styles.iconOnly, styles.purple)}>
                                <Mark name="replyAll" size={17} />
                            </span>
                            <span {...style.attrs(styles.command, styles.iconOnly, styles.purple)}>
                                <Mark name="forward" size={17} />
                            </span>
                            <span {...style.attrs(styles.divider)} />
                            <span {...style.attrs(styles.command, styles.wide)}>
                                <span {...style.attrs(styles.amber)}>
                                    <Mark name="bolt" size={16} />
                                </span>
                                Quick steps
                            </span>
                            <span {...style.attrs(styles.command, styles.wide)}>
                                <Mark name="read" size={16} />
                                Read / Unread
                            </span>
                            <span {...style.attrs(styles.command, styles.iconOnly, styles.red)}>
                                <Mark name="flag" size={17} />
                            </span>
                            <Mark name="more" size={16} />
                        </span>
                    </div>

                    <div {...style.attrs(styles.panes)}>
                        {/* folders */}
                        <nav {...style.attrs(styles.folders)}>
                            <span {...style.attrs(styles.section)}>
                                <Mark name="caret" size={12} />
                                Favorites
                            </span>
                            <span {...style.attrs(styles.folder)}>
                                <Mark name="inbox" size={16} />
                                Inbox
                                <span {...style.attrs(styles.count)}>2</span>
                            </span>
                            <span {...style.attrs(styles.section)}>
                                <Mark name="caret" size={12} />
                                florian@symbol.co
                            </span>
                            {folders.map(([name, glyph, count], index) => (
                                <span
                                    {...style.attrs(styles.folder, index === 0 && styles.folderOn)}
                                >
                                    <Mark name={glyph} size={16} />
                                    {name}
                                    <span {...style.attrs(styles.count)}>{count}</span>
                                </span>
                            ))}
                        </nav>

                        {/* messages */}
                        <section {...style.attrs(styles.list)}>
                            <span {...style.attrs(styles.listHead)}>
                                <b {...style.attrs(styles.listTitle)}>Inbox</b>
                                <Mark name="filter" size={16} />
                            </span>
                            <span {...style.attrs(styles.pivots)}>
                                <span {...style.attrs(styles.pivotOn)}>Focused</span>
                                <span>Other</span>
                            </span>
                            <span {...style.attrs(styles.group)}>Today</span>
                            {mails
                                .slice(0, 3)
                                .map(([sender, tint, subject, preview, time, isUnread], index) => (
                                    <MessageRow
                                        isSelected={index === 0}
                                        sender={sender}
                                        tint={tint}
                                        subject={subject}
                                        preview={preview}
                                        time={time}
                                        isUnread={isUnread}
                                    />
                                ))}
                            <span {...style.attrs(styles.group)}>Yesterday</span>
                            {mails
                                .slice(3, 5)
                                .map(([sender, tint, subject, preview, time, isUnread]) => (
                                    <MessageRow
                                        isSelected={false}
                                        sender={sender}
                                        tint={tint}
                                        subject={subject}
                                        preview={preview}
                                        time={time}
                                        isUnread={isUnread}
                                    />
                                ))}
                        </section>

                        {/* the new message */}
                        <section {...style.attrs(styles.compose)}>
                            <span {...style.attrs(styles.composeTools)}>
                                <span {...style.attrs(styles.send)}>
                                    Send
                                    <span {...style.attrs(styles.sendCaret)}>
                                        <Mark name="caret" size={12} />
                                    </span>
                                </span>
                                <Mark name="attach" size={16} />
                                <span {...style.attrs(styles.discard)}>Discard</span>
                            </span>
                            <span {...style.attrs(styles.warningBar)}>
                                <Mark name="warning" size={16} />
                                This message can't be sent. It has 1,840 recipients, and the limit
                                is 500.
                            </span>
                            <span {...style.attrs(styles.field)}>
                                <span {...style.attrs(styles.fieldName)}>To</span>
                                <span {...style.attrs(styles.recipient)}>Maya Chen</span>
                                <span {...style.attrs(styles.recipient)}>Jonas Weber</span>
                                <span {...style.attrs(styles.recipient)}>Tara Okafor</span>
                                <span {...style.attrs(styles.more)}>+1,837</span>
                            </span>
                            <span {...style.attrs(styles.field, styles.subject)}>
                                Destack opens Friday
                            </span>
                            <p {...style.attrs(styles.text)}>
                                Hi there, Destack opens on Friday, and you're in. Your invite link
                                works from 09:00, and early pricing holds for everyone on the
                                waitlist.
                            </p>
                        </section>
                    </div>
                </div>
            </div>
        </div>
    );
}

/** Draw one message of the list: its sender, subject, preview and time, marked while unread. */
function MessageRow(properties: {
    isSelected: boolean;
    sender: string;
    tint: string;
    subject: string;
    preview: string;
    time: string;
    isUnread: boolean;
}) {
    return (
        <span
            {...style.attrs(
                styles.row,
                properties.isUnread && styles.rowUnread,
                properties.isSelected && styles.rowSelected,
            )}
        >
            <span style={{ "background-color": properties.tint }} {...style.attrs(styles.avatar)}>
                {properties.sender.slice(0, 1)}
            </span>
            <span {...style.attrs(styles.rowText)}>
                <span {...style.attrs(styles.rowTop)}>
                    <span {...style.attrs(properties.isUnread && styles.bold)}>
                        {properties.sender}
                    </span>
                    <span {...style.attrs(styles.time, properties.isUnread && styles.timeUnread)}>
                        {properties.time}
                    </span>
                </span>
                <span {...style.attrs(properties.isUnread && styles.subjectUnread)}>
                    {properties.subject}
                </span>
                <span {...style.attrs(styles.preview)}>{properties.preview}</span>
            </span>
        </span>
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

/** The Outlook page styles. */
const styles = style.create({
    page: {
        backgroundColor: outlook.frame,
        color: outlook.ink,
        display: "grid",
        fontFamily: outlook.face,
        fontSize: "14px",
        gridTemplateColumns: "minmax(0, 1fr)",
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
        strokeWidth: 1.6,
    },

    // header bar
    header: {
        alignItems: "center",
        backgroundColor: outlook.header,
        color: outlook.muted,
        display: "flex",
        gap: "16px",
        height: "48px",
        paddingInline: "14px",
    },
    brand: {
        alignItems: "center",
        color: outlook.ink,
        display: "flex",
        fontSize: "14px",
        fontWeight: 600,
        gap: "8px",
    },
    search: {
        alignItems: "center",
        backgroundColor: outlook.search,
        borderColor: outlook.searchLine,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: outlook.muted,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "8px",
        height: "32px",
        marginInline: "auto",
        maxWidth: "468px",
        paddingInline: "10px",
        width: "50%",
    },
    headerEnd: {
        alignItems: "center",
        display: "flex",
        gap: "16px",
        marginInlineStart: { default: "0", [site.narrow]: "auto" },
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: outlook.onAccent,
        display: "inline-flex",
        fontSize: "12px",
        height: "30px",
        justifyContent: "center",
        width: "30px",
    },
    body: {
        display: "grid",
        gridTemplateColumns: { default: "48px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },

    // app rail
    rail: {
        alignItems: "center",
        color: outlook.muted,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "4px",
        paddingBlockStart: "8px",
    },
    app: {
        alignItems: "center",
        borderRadius: "4px",
        display: "inline-flex",
        height: "40px",
        justifyContent: "center",
        position: "relative",
        width: "40px",
    },
    appOn: {
        color: outlook.blue,
        "::before": {
            backgroundColor: outlook.blue,
            borderRadius: "2px",
            content: '""',
            height: "20px",
            insetInlineStart: "-4px",
            position: "absolute",
            width: "3px",
        },
    },
    work: {
        display: "grid",
        gap: "8px",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: 0,
        paddingBlock: { default: "8px", [site.narrow]: "0" },
        paddingInlineEnd: { default: "8px", [site.narrow]: "0" },
    },

    // ribbon
    ribbon: {
        backgroundColor: outlook.surface,
        borderRadius: "6px",
        display: "grid",
        paddingInline: "8px",
    },
    ribbonTabs: {
        color: outlook.muted,
        display: "flex",
        gap: "20px",
        paddingBlockEnd: "0",
        paddingBlockStart: "6px",
        paddingInline: "6px",
    },
    ribbonTabOn: {
        boxShadow: `inset 0 -2px 0 ${outlook.blue}`,
        color: outlook.ink,
        fontWeight: 600,
        paddingBlockEnd: "6px",
    },
    commands: {
        alignItems: "center",
        color: outlook.ink,
        display: "flex",
        gap: "4px",
        overflow: "hidden",
        paddingBlock: "6px",
        whiteSpace: "nowrap",
    },
    newMail: {
        alignItems: "center",
        backgroundColor: outlook.blue,
        borderRadius: "4px",
        color: outlook.onAccent,
        display: "inline-flex",
        fontWeight: 600,
        gap: "6px",
        height: "32px",
        marginInlineEnd: "8px",
        paddingInlineStart: "10px",
    },
    newMailCaret: {
        alignItems: "center",
        borderInlineStartColor: outlook.split,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        display: "inline-flex",
        height: "100%",
        paddingInline: "6px",
    },
    command: {
        alignItems: "center",
        borderRadius: "4px",
        display: "inline-flex",
        gap: "6px",
        height: "32px",
        paddingInline: "8px",
    },
    wide: {
        display: { default: "inline-flex", [site.narrow]: "none" },
    },
    iconOnly: {
        paddingInline: "6px",
    },
    divider: {
        backgroundColor: outlook.line,
        flexShrink: 0,
        height: "20px",
        marginInline: "4px",
        width: "1px",
    },
    red: {
        color: outlook.red,
    },
    purple: {
        color: outlook.purple,
    },
    amber: {
        color: outlook.amber,
        display: "inline-flex",
    },
    green: {
        color: outlook.green,
    },
    panes: {
        display: "grid",
        gap: "8px",
        gridTemplateColumns: {
            default: "190px 260px minmax(0, 1fr)",
            [site.narrow]: "minmax(0, 1fr)",
        },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },

    // folders
    folders: {
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "1px",
    },
    section: {
        alignItems: "center",
        display: "flex",
        fontWeight: 600,
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "6px",
    },
    folder: {
        alignItems: "center",
        borderRadius: "4px",
        display: "flex",
        gap: "10px",
        paddingBlock: "6px",
        paddingInlineEnd: "10px",
        paddingInlineStart: "24px",
    },
    folderOn: {
        backgroundColor: outlook.selected,
        fontWeight: 600,
    },
    count: {
        color: outlook.blue,
        fontWeight: 600,
        marginInlineStart: "auto",
    },

    // messages
    list: {
        backgroundColor: outlook.surface,
        borderRadius: "6px",
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        overflow: "hidden",
    },
    listHead: {
        alignItems: "center",
        color: outlook.muted,
        display: "flex",
        justifyContent: "space-between",
        paddingBlockEnd: "4px",
        paddingBlockStart: "10px",
        paddingInline: "14px",
    },
    listTitle: {
        color: outlook.ink,
        fontSize: "16px",
    },
    pivots: {
        borderBottomColor: outlook.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: outlook.muted,
        display: "flex",
        gap: "16px",
        paddingInline: "14px",
    },
    pivotOn: {
        boxShadow: `inset 0 -2px 0 ${outlook.blue}`,
        color: outlook.ink,
        fontWeight: 600,
        paddingBlockEnd: "6px",
    },
    group: {
        color: outlook.muted,
        fontSize: "12px",
        fontWeight: 600,
        paddingBlockEnd: "4px",
        paddingBlockStart: "8px",
        paddingInline: "14px",
    },
    row: {
        borderInlineStartColor: "transparent",
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "3px",
        display: "flex",
        gap: "10px",
        paddingBlock: "8px",
        paddingInlineEnd: "14px",
        paddingInlineStart: "11px",
    },
    rowUnread: {
        borderInlineStartColor: outlook.blue,
    },
    rowSelected: {
        backgroundColor: outlook.selected,
    },
    avatar: {
        alignItems: "center",
        borderRadius: "50%",
        color: outlook.onAccent,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "12px",
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },
    rowText: {
        display: "grid",
        flexGrow: 1,
        fontSize: "13px",
        gridTemplateColumns: "minmax(0, 1fr)",
        lineHeight: 1.4,
        minWidth: 0,
    },
    rowTop: {
        display: "flex",
        justifyContent: "space-between",
    },
    bold: {
        fontWeight: 700,
    },
    time: {
        color: outlook.muted,
        fontSize: "12px",
    },
    timeUnread: {
        color: outlook.blue,
        fontWeight: 600,
    },
    subjectUnread: {
        color: outlook.blue,
        fontWeight: 600,
    },
    preview: {
        color: outlook.muted,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },

    // the new message
    compose: {
        alignContent: "start",
        backgroundColor: outlook.surface,
        borderRadius: "6px",
        display: "grid",
        gap: "10px",
        paddingBlock: "12px",
        paddingInline: "16px",
    },
    composeTools: {
        alignItems: "center",
        color: outlook.muted,
        display: "flex",
        gap: "14px",
    },
    send: {
        alignItems: "center",
        backgroundColor: outlook.blue,
        borderRadius: "4px",
        color: outlook.onAccent,
        display: "inline-flex",
        fontWeight: 600,
        gap: "8px",
        height: "32px",
        paddingInlineStart: "14px",
    },
    sendCaret: {
        alignItems: "center",
        borderInlineStartColor: outlook.split,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        display: "inline-flex",
        height: "100%",
        paddingInline: "6px",
    },
    discard: {
        borderColor: outlook.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: outlook.ink,
        paddingBlock: "5px",
        paddingInline: "12px",
    },
    warningBar: {
        alignItems: "center",
        backgroundColor: outlook.warning,
        borderRadius: "4px",
        color: outlook.warningInk,
        display: "flex",
        fontSize: "13px",
        gap: "8px",
        lineHeight: 1.4,
        paddingBlock: "8px",
        paddingInline: "10px",
    },
    field: {
        alignItems: "center",
        borderBottomColor: outlook.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        flexWrap: "wrap",
        gap: "6px",
        paddingBlock: "6px",
    },
    fieldName: {
        color: outlook.muted,
        marginInlineEnd: "6px",
    },
    recipient: {
        backgroundColor: outlook.selected,
        borderRadius: "4px",
        fontSize: "13px",
        paddingBlock: "1px",
        paddingInline: "6px",
    },
    more: {
        color: outlook.blue,
        fontSize: "13px",
        fontWeight: 600,
    },
    subject: {
        fontSize: "15px",
    },
    text: {
        lineHeight: 1.5,
        margin: 0,
    },
});
