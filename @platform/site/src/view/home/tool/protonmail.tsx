import * as style from "@destack/style";

import { mails } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { protonMail } from "./protonmail.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    inbox: ["M4 13h4.5l1.5 3h4l1.5-3H20", "M6 5h12l2 8v6H4v-6z"],
    draft: ["M5 4h10l4 4v12H5z", "m9 15 5-5 2 2-5 5H9z"],
    sent: ["M4 12 20 4l-4 16-4-7z", "m12 13 8-9"],
    star: ["m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"],
    archive: ["M4 5h16v4H4z", "M5 9v10h14V9", "M10 13h4"],
    spam: ["M12 3 4 6v6c0 5 3.5 8 8 9 4.5-1 8-4 8-9V6z", "M12 8v5", "M12 16h.01"],
    trash: ["M5 7h14", "M9 7V4h6v3", "M7 7l1 13h8l1-13"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    sliders: ["M4 7h10", "M18 7h2", "M4 17h4", "M12 17h8", "M16 5v4", "M10 15v4"],
    help: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6",
        "M12 17h.01",
    ],
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
    apps: ["M5 5h5v5H5z", "M14 5h5v5h-5z", "M5 14h5v5H5z", "M14 14h5v5h-5z"],
    refresh: ["M20 12a8 8 0 1 1-2.3-5.7L20 8", "M20 4v4h-4"],
    caret: ["m7 10 5 5 5-5"],
    minimize: ["M6 12h12"],
    expand: ["M4 9V4h5", "M20 15v5h-5"],
    close: ["M18 6 6 18", "m6 6 12 12"],
    attach: ["m20 11-8.5 8.5a5 5 0 0 1-7-7L13 4a3.5 3.5 0 0 1 5 5l-8.5 8.5a2 2 0 0 1-3-3L14 7"],
    lock: ["M7 11V7a5 5 0 0 1 10 0v4", "M5 11h14v10H5z"],
    alert: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 7v6", "M12 16.5h.01"],
} satisfies Record<string, readonly string[]>;

/** The folders the sidebar lists: name, glyph and unread count, the inbox chosen. */
const folders: readonly (readonly [name: string, glyph: keyof typeof glyphs, count: string])[] = [
    ["Inbox", "inbox", "2"],
    ["Drafts", "draft", "1"],
    ["Sent", "sent", ""],
    ["Starred", "star", ""],
    ["Archive", "archive", ""],
    ["Spam", "spam", ""],
    ["Trash", "trash", ""],
];

/** The labels the sidebar lists: name and colour. */
const labels: readonly (readonly [name: string, tint: string])[] = [
    ["Launch", "#6d4aff"],
    ["Press", "#e5813b"],
];

/** Draw the launch mail in the encrypted inbox, its composer refusing the waitlist send. */
export function ProtonMail() {
    return (
        <div data-component="ProtonMailPage" {...style.attrs(styles.page)}>
            {/* sidebar */}
            <nav {...style.attrs(styles.side)}>
                <span {...style.attrs(styles.brand)}>
                    <Favicon icon="protonmail" tint="#6d4aff" size={26} />
                    <span>
                        <b>Proton</b> Mail
                    </span>
                </span>
                <span {...style.attrs(styles.newMessage)}>New message</span>
                {folders.map(([name, glyph, count], index) => (
                    <span {...style.attrs(styles.folder, index === 0 && styles.folderOn)}>
                        <Mark name={glyph} size={16} />
                        <span {...style.attrs(count !== "" && styles.bold)}>{name}</span>
                        <span {...style.attrs(styles.count)}>{count}</span>
                    </span>
                ))}
                <span {...style.attrs(styles.section)}>
                    Labels
                    <Mark name="caret" size={12} />
                </span>
                {labels.map(([name, tint]) => (
                    <span {...style.attrs(styles.folder)}>
                        <span style={{ "background-color": tint }} {...style.attrs(styles.label)} />
                        {name}
                    </span>
                ))}
                <span {...style.attrs(styles.storage)}>
                    <span {...style.attrs(styles.storageBar)}>
                        <span {...style.attrs(styles.storageFill)} />
                    </span>
                    1.21 GB of 15 GB
                </span>
            </nav>

            <div {...style.attrs(styles.main)}>
                {/* header */}
                <header {...style.attrs(styles.header)}>
                    <span {...style.attrs(styles.search)}>
                        <Mark name="search" size={16} />
                        Search messages
                        <span {...style.attrs(styles.searchEnd)}>
                            <Mark name="sliders" size={16} />
                        </span>
                    </span>
                    <span {...style.attrs(styles.headerEnd)}>
                        <Mark name="help" size={18} />
                        <Mark name="gear" size={18} />
                        <Mark name="apps" size={18} />
                        <span {...style.attrs(styles.user)}>
                            <span {...style.attrs(styles.userText)}>
                                <b>Florian</b>
                                <span>florian@symbol.co</span>
                            </span>
                            <span {...style.attrs(styles.initials)}>FC</span>
                        </span>
                    </span>
                </header>

                {/* messages */}
                <section {...style.attrs(styles.list)}>
                    <span {...style.attrs(styles.toolbar)}>
                        <span {...style.attrs(styles.checkbox)} />
                        <Mark name="caret" size={12} />
                        <Mark name="refresh" size={16} />
                        <span {...style.attrs(styles.toolbarEnd)}>1 / 1</span>
                    </span>
                    {mails.map(([sender, tint, subject, , time, isUnread]) => (
                        <span {...style.attrs(styles.row, !isUnread && styles.rowRead)}>
                            <span
                                style={{ "background-color": tint }}
                                {...style.attrs(styles.avatar)}
                            >
                                {sender.slice(0, 2).toUpperCase()}
                            </span>
                            <span {...style.attrs(styles.rowText)}>
                                <span {...style.attrs(styles.rowTop)}>
                                    <span {...style.attrs(isUnread && styles.bold)}>{sender}</span>
                                    <span {...style.attrs(styles.time, isUnread && styles.bold)}>
                                        {time}
                                    </span>
                                </span>
                                <span {...style.attrs(styles.subject, isUnread && styles.bold)}>
                                    {subject}
                                </span>
                            </span>
                        </span>
                    ))}

                    {/* composer */}
                    <section {...style.attrs(styles.composer)}>
                        <span {...style.attrs(styles.composerHead)}>
                            <b>Destack opens Friday</b>
                            <span {...style.attrs(styles.composerTools)}>
                                <Mark name="minimize" size={14} />
                                <Mark name="expand" size={14} />
                                <Mark name="close" size={14} />
                            </span>
                        </span>
                        <span {...style.attrs(styles.field)}>
                            <span {...style.attrs(styles.fieldName)}>To</span>
                            <span {...style.attrs(styles.recipient)}>
                                <Mark name="lock" size={10} />
                                Maya Chen
                            </span>
                            <span {...style.attrs(styles.recipient)}>Jonas Weber</span>
                            <span {...style.attrs(styles.recipientMore)}>+1,838</span>
                        </span>
                        <span {...style.attrs(styles.error)}>
                            <Mark name="alert" size={14} />
                            1,840 recipients. A message can have 100 at most.
                        </span>
                        <p {...style.attrs(styles.text)}>
                            Hi there, Destack opens on Friday, and you're in. Your invite link works
                            from 09:00.
                        </p>
                        <span {...style.attrs(styles.composerFoot)}>
                            <span {...style.attrs(styles.send)}>Send</span>
                            <Mark name="attach" size={16} />
                            <Mark name="lock" size={16} />
                        </span>
                    </section>
                </section>
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

/** The Proton Mail page styles. */
const styles = style.create({
    page: {
        backgroundColor: protonMail.chrome,
        color: protonMail.ink,
        display: "grid",
        fontFamily: protonMail.face,
        fontSize: "14px",
        gridTemplateColumns: { default: "232px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
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
    bold: {
        fontWeight: 600,
    },

    // sidebar
    side: {
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "1px",
        paddingBlock: "14px",
        paddingInline: "12px",
    },
    brand: {
        alignItems: "center",
        color: protonMail.purple,
        display: "flex",
        fontSize: "19px",
        gap: "8px",
        paddingBlockEnd: "16px",
        paddingInline: "4px",
    },
    newMessage: {
        backgroundColor: protonMail.purple,
        borderRadius: "8px",
        color: protonMail.onAccent,
        fontWeight: 600,
        marginBlockEnd: "12px",
        paddingBlock: "10px",
        textAlign: "center",
    },
    folder: {
        alignItems: "center",
        borderRadius: "8px",
        color: protonMail.ink,
        display: "flex",
        gap: "10px",
        paddingBlock: "6px",
        paddingInline: "10px",
    },
    folderOn: {
        backgroundColor: protonMail.selected,
        fontWeight: 600,
    },
    count: {
        fontWeight: 600,
        marginInlineStart: "auto",
    },
    section: {
        alignItems: "center",
        color: protonMail.muted,
        display: "flex",
        fontSize: "12px",
        fontWeight: 600,
        gap: "4px",
        paddingBlockEnd: "4px",
        paddingBlockStart: "14px",
        paddingInline: "10px",
    },
    label: {
        borderRadius: "50%",
        height: "10px",
        marginInline: "3px",
        width: "10px",
    },
    storage: {
        color: protonMail.muted,
        display: "grid",
        fontSize: "12px",
        gap: "6px",
        marginBlockStart: "auto",
        paddingInline: "10px",
    },
    storageBar: {
        backgroundColor: protonMail.field,
        borderRadius: "2px",
        height: "4px",
        overflow: "hidden",
    },
    storageFill: {
        backgroundColor: protonMail.purple,
        display: "block",
        height: "100%",
        width: "8%",
    },
    main: {
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minWidth: 0,
    },

    // header
    header: {
        alignItems: "center",
        display: "flex",
        gap: "16px",
        height: "60px",
        paddingInline: "16px",
    },
    search: {
        alignItems: "center",
        backgroundColor: protonMail.field,
        borderRadius: "8px",
        color: protonMail.muted,
        display: "flex",
        flexGrow: 1,
        gap: "10px",
        height: "38px",
        maxWidth: "440px",
        paddingInline: "12px",
    },
    searchEnd: {
        display: "flex",
        marginInlineStart: "auto",
    },
    headerEnd: {
        alignItems: "center",
        color: protonMail.muted,
        display: "flex",
        gap: "16px",
        marginInlineStart: "auto",
    },
    user: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
    },
    userText: {
        color: protonMail.ink,
        display: { default: "grid", [site.narrow]: "none" },
        fontSize: "12px",
        lineHeight: 1.3,
        textAlign: "end",
    },
    initials: {
        alignItems: "center",
        backgroundColor: protonMail.purple,
        borderRadius: "8px",
        color: protonMail.onAccent,
        display: "inline-flex",
        fontSize: "12px",
        fontWeight: 600,
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },

    // messages
    list: {
        backgroundColor: protonMail.surface,
        borderStartStartRadius: { default: "12px", [site.narrow]: "0" },
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        position: "relative",
    },
    toolbar: {
        alignItems: "center",
        borderBottomColor: protonMail.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: protonMail.muted,
        display: "flex",
        gap: "10px",
        paddingBlock: "10px",
        paddingInline: "16px",
    },
    checkbox: {
        borderColor: protonMail.muted,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        height: "14px",
        width: "14px",
    },
    toolbarEnd: {
        fontSize: "12px",
        marginInlineStart: "auto",
    },
    row: {
        alignItems: "center",
        borderBottomColor: protonMail.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "12px",
        paddingBlock: "10px",
        paddingInline: "16px",
    },
    rowRead: {
        backgroundColor: protonMail.read,
        color: protonMail.muted,
    },
    avatar: {
        alignItems: "center",
        borderRadius: "8px",
        color: protonMail.onAccent,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "11px",
        fontWeight: 600,
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },
    rowText: {
        display: "grid",
        flexGrow: 1,
        gridTemplateColumns: "minmax(0, 1fr)",
        minWidth: 0,
    },
    rowTop: {
        display: "flex",
        justifyContent: "space-between",
    },
    time: {
        fontSize: "12px",
    },
    subject: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },

    // composer
    composer: {
        backgroundColor: protonMail.surface,
        borderRadius: "12px",
        bottom: "0",
        boxShadow: protonMail.lift,
        display: "grid",
        gap: "8px",
        insetInlineEnd: "16px",
        paddingBlockEnd: "12px",
        position: "absolute",
        width: { default: "380px", [site.narrow]: "calc(100% - 32px)" },
    },
    composerHead: {
        alignItems: "center",
        backgroundColor: protonMail.chrome,
        borderStartEndRadius: "12px",
        borderStartStartRadius: "12px",
        display: "flex",
        paddingBlock: "10px",
        paddingInline: "14px",
    },
    composerTools: {
        color: protonMail.muted,
        display: "flex",
        gap: "12px",
        marginInlineStart: "auto",
    },
    field: {
        alignItems: "center",
        borderBottomColor: protonMail.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        flexWrap: "wrap",
        gap: "6px",
        marginInline: "14px",
        paddingBlock: "6px",
    },
    fieldName: {
        color: protonMail.muted,
        marginInlineEnd: "4px",
    },
    recipient: {
        alignItems: "center",
        backgroundColor: protonMail.selected,
        borderRadius: "6px",
        display: "inline-flex",
        fontSize: "13px",
        gap: "4px",
        paddingBlock: "2px",
        paddingInline: "8px",
    },
    recipientMore: {
        color: protonMail.purple,
        fontSize: "13px",
        fontWeight: 600,
    },
    error: {
        alignItems: "center",
        backgroundColor: protonMail.dangerTint,
        borderRadius: "8px",
        color: protonMail.danger,
        display: "flex",
        fontSize: "13px",
        gap: "8px",
        marginInline: "14px",
        paddingBlock: "8px",
        paddingInline: "10px",
    },
    text: {
        lineHeight: 1.5,
        margin: 0,
        paddingInline: "14px",
    },
    composerFoot: {
        alignItems: "center",
        color: protonMail.muted,
        display: "flex",
        gap: "14px",
        paddingInline: "14px",
    },
    send: {
        backgroundColor: protonMail.purple,
        borderRadius: "8px",
        color: protonMail.onAccent,
        fontWeight: 600,
        opacity: 0.5,
        paddingBlock: "7px",
        paddingInline: "18px",
    },
});
