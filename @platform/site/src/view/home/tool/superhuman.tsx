import * as style from "@destack/style";

import { mails } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { superhuman } from "./superhuman.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    menu: ["M4 7h16", "M4 12h16", "M4 17h16"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    warning: ["M12 3 2 20h20z", "M12 9v5", "M12 17h.01"],
} satisfies Record<string, readonly string[]>;

/** The inbox's splits: name and count, Important chosen. */
const splits: readonly (readonly [name: string, count: string])[] = [
    ["Important", "2"],
    ["Calendar", "1"],
    ["Team", ""],
    ["News", ""],
    ["Other", "4"],
];

/** Draw the launch mail in the fast client, its Gmail account refusing the waitlist send. */
export function Superhuman() {
    return (
        <div data-component="SuperhumanPage" {...style.attrs(styles.page)}>
            {/* splits */}
            <header {...style.attrs(styles.head)}>
                <Mark name="menu" size={16} />
                <span {...style.attrs(styles.splits)}>
                    {splits.map(([name, count], index) => (
                        <span
                            {...style.attrs(
                                styles.split,
                                index === 0 && styles.splitOn,
                                index > 2 && styles.spare,
                            )}
                        >
                            {name}
                            {count === "" ? undefined : (
                                <span {...style.attrs(styles.count)}>{count}</span>
                            )}
                        </span>
                    ))}
                </span>
                <span {...style.attrs(styles.headEnd)}>
                    <Mark name="search" size={16} />
                    <span {...style.attrs(styles.logo)}>
                        <Favicon icon="superhuman" tint="#ffffff" size={14} />
                    </span>
                </span>
            </header>

            {/* messages */}
            <ul {...style.attrs(styles.list)}>
                {mails.map(([sender, , subject, preview, time, isUnread], index) => (
                    <li {...style.attrs(styles.row, index === 0 && styles.rowOn)}>
                        <span {...style.attrs(styles.sender, isUnread && styles.unread)}>
                            {sender}
                        </span>
                        <span {...style.attrs(styles.line)}>
                            <span {...style.attrs(isUnread && styles.unread)}>{subject}</span>
                            <span {...style.attrs(styles.preview)}>{preview}</span>
                        </span>
                        <span {...style.attrs(styles.time)}>{time}</span>
                    </li>
                ))}
            </ul>

            {/* compose */}
            <section {...style.attrs(styles.compose)}>
                <span {...style.attrs(styles.field)}>
                    <span {...style.attrs(styles.fieldName)}>To</span>
                    maya@northwind.io, jonas@kettle.dev, tara@fieldnotes.co
                    <span {...style.attrs(styles.more)}>+1,837</span>
                </span>
                <span {...style.attrs(styles.field, styles.subject)}>Destack opens Friday</span>
                <p {...style.attrs(styles.text)}>
                    Hi there, Destack opens on Friday, and you're in. Your invite link works from
                    09:00.
                </p>
                <span {...style.attrs(styles.composeFoot)}>
                    <span {...style.attrs(styles.send)}>Send</span>
                    <span {...style.attrs(styles.hint)}>⌘ Enter</span>
                </span>
            </section>

            {/* the refusal */}
            <span role="alert" {...style.attrs(styles.toast)}>
                <span {...style.attrs(styles.toastMark)}>
                    <Mark name="warning" size={15} />
                </span>
                Gmail didn't send this: 1,840 recipients, and a message takes 500 outside your
                domain
            </span>
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

/** The Superhuman page styles. */
const styles = style.create({
    page: {
        backgroundColor: superhuman.surface,
        color: superhuman.ink,
        display: "grid",
        fontFamily: superhuman.face,
        fontSize: "13px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
        position: "relative",
    },
    mark: {
        fill: "none",
        flexShrink: 0,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.8,
    },
    head: {
        alignItems: "center",
        color: superhuman.muted,
        display: "flex",
        gap: "18px",
        height: "54px",
        paddingInline: "24px",
    },
    splits: {
        display: "flex",
        gap: "22px",
    },
    split: {
        alignItems: "center",
        display: "flex",
        gap: "6px",
        paddingBlock: "17px",
    },
    splitOn: {
        boxShadow: `inset 0 -2px 0 ${superhuman.split}`,
        color: superhuman.split,
        fontWeight: 600,
    },
    spare: {
        display: { default: "flex", [site.narrow]: "none" },
    },
    count: {
        fontSize: "11px",
        opacity: 0.8,
    },
    headEnd: {
        alignItems: "center",
        display: "flex",
        gap: "16px",
        marginInlineStart: "auto",
    },
    logo: {
        alignItems: "center",
        backgroundColor: superhuman.plum,
        borderRadius: "8px",
        display: "inline-flex",
        height: "26px",
        justifyContent: "center",
        width: "26px",
    },
    list: {
        listStyle: "none",
        margin: 0,
        overflow: "hidden",
        padding: 0,
    },
    row: {
        alignItems: "baseline",
        borderBottomColor: superhuman.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: superhuman.muted,
        display: "grid",
        gap: "16px",
        gridTemplateColumns: {
            default: "150px minmax(0, 1fr) 54px",
            [site.narrow]: "100px minmax(0, 1fr) 46px",
        },
        paddingBlock: "11px",
        paddingInline: "24px",
    },
    rowOn: {
        backgroundColor: superhuman.selected,
        boxShadow: `inset 3px 0 0 ${superhuman.split}`,
    },
    sender: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    unread: {
        color: superhuman.ink,
        fontWeight: 600,
    },
    line: {
        display: "flex",
        gap: "10px",
        minWidth: 0,
        overflow: "hidden",
        whiteSpace: "nowrap",
    },
    preview: {
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    time: {
        fontSize: "12px",
        textAlign: "end",
    },
    compose: {
        backgroundColor: superhuman.panel,
        borderRadius: "10px",
        bottom: "54px",
        boxShadow: superhuman.lift,
        display: "grid",
        gap: "8px",
        insetInlineEnd: "24px",
        padding: "16px",
        position: "absolute",
        width: { default: "380px", [site.narrow]: "calc(100% - 48px)" },
    },
    field: {
        borderBottomColor: superhuman.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "8px",
        overflow: "hidden",
        paddingBlockEnd: "8px",
        whiteSpace: "nowrap",
    },
    fieldName: {
        color: superhuman.muted,
    },
    more: {
        color: superhuman.split,
        fontWeight: 600,
    },
    subject: {
        fontWeight: 600,
    },
    text: {
        lineHeight: 1.5,
        margin: 0,
    },
    composeFoot: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
    },
    send: {
        backgroundColor: superhuman.plum,
        borderRadius: "6px",
        color: superhuman.onAccent,
        fontWeight: 600,
        paddingBlock: "6px",
        paddingInline: "14px",
    },
    hint: {
        color: superhuman.muted,
        fontSize: "12px",
    },
    toast: {
        alignItems: "center",
        backgroundColor: superhuman.toast,
        borderRadius: "8px",
        bottom: "12px",
        color: superhuman.onAccent,
        display: "flex",
        fontSize: "12px",
        gap: "8px",
        insetInlineStart: "50%",
        maxWidth: "calc(100% - 32px)",
        paddingBlock: "8px",
        paddingInline: "12px",
        position: "absolute",
        transform: "translateX(-50%)",
        whiteSpace: { default: "nowrap", [site.narrow]: "normal" },
        width: { default: "auto", [site.narrow]: "calc(100% - 32px)" },
    },
    toastMark: {
        color: superhuman.danger,
        display: "flex",
    },
});
