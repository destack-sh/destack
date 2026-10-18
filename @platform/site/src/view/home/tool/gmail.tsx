import * as style from "@destack/style";

import { mails } from "../verb/corner";
import { Favicon, Glyph } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { palette } from "../../palette.stylex";
import { gmail } from "./gmail.stylex";

/** Draw the launch mail as a rented inbox, with the waitlist pasted into a new message. */
export function Gmail() {
    return (
        <div data-component="GmailPage" {...style.attrs(styles.gmail)}>
            <nav {...style.attrs(styles.gmBar)}>
                <span {...style.attrs(styles.gmBrand)}>
                    <Glyph name="menu" size={20} />
                    <Favicon icon="gmail" tint="#ea4335" size={22} />
                    Gmail
                </span>
                <span {...style.attrs(styles.gmSearch)}>
                    <Glyph name="search" size={16} />
                    Search mail
                </span>
                <span {...style.attrs(styles.gmFace)}>F</span>
            </nav>
            <div {...style.attrs(styles.gmFrame)}>
                <nav {...style.attrs(styles.gmSide)}>
                    <span {...style.attrs(styles.gmCompose)}>
                        <Glyph name="compose" size={18} />
                        Compose
                    </span>
                    {[
                        ["Inbox", "2"],
                        ["Starred", ""],
                        ["Snoozed", ""],
                        ["Sent", ""],
                        ["Drafts", "1"],
                    ].map(([folder, count], index) => (
                        <span {...style.attrs(styles.gmFolder, index === 0 && styles.gmFolderOn)}>
                            {folder}
                            <span>{count}</span>
                        </span>
                    ))}
                </nav>
                <ul {...style.attrs(styles.gmList)}>
                    {mails.map(([sender, tint, subject, preview, time, isUnread]) => (
                        <li {...style.attrs(styles.gmRow, !isUnread && styles.gmRowRead)}>
                            <span
                                style={{ "background-color": tint }}
                                {...style.attrs(styles.gmAvatar)}
                            >
                                {sender.slice(0, 1)}
                            </span>
                            <span {...style.attrs(styles.gmSender, isUnread && styles.gmUnread)}>
                                {sender}
                            </span>
                            <span {...style.attrs(styles.gmLine)}>
                                <span {...style.attrs(isUnread && styles.gmUnread)}>{subject}</span>
                                <span {...style.attrs(styles.gmPreview)}> – {preview}</span>
                            </span>
                            <span {...style.attrs(styles.gmTime, isUnread && styles.gmUnread)}>
                                {time}
                            </span>
                        </li>
                    ))}
                </ul>
            </div>
            <div data-component="ComposeWindow" {...style.attrs(styles.gmWindow)}>
                <span {...style.attrs(styles.gmWindowHead)}>New Message</span>
                <span {...style.attrs(styles.gmField)}>
                    <span {...style.attrs(styles.gmPreview)}>To</span>
                    maya@northwind.io, jonas@kettle.dev, tara@fieldnotes.co, +1,837
                </span>
                <span {...style.attrs(styles.gmField)}>Destack opens Friday</span>
                <p {...style.attrs(styles.gmBody)}>
                    Hi there, Destack opens on Friday, and you're in. Your invite link works from
                    09:00, and early pricing holds for everyone on the waitlist.
                </p>
                <span {...style.attrs(styles.gmWarning)}>
                    1,840 recipients: over the limit of 500 external recipients a message
                </span>
                <span {...style.attrs(styles.gmTools)}>
                    <span {...style.attrs(styles.gmSend)}>Send</span>
                    <Glyph name="link" size={18} />
                    <Glyph name="file" size={18} />
                    <Glyph name="more" size={18} />
                </span>
            </div>
        </div>
    );
}

/** The gmail page styles. */
const styles = style.create({
    gmail: {
        backgroundColor: gmail.canvas,
        color: gmail.ink,
        display: "grid",
        fontFamily: gmail.face,
        fontSize: "14px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
        position: "relative",
    },
    gmBar: {
        alignItems: "center",
        display: "flex",
        gap: { default: "24px", [site.narrow]: "10px" },
        height: "64px",
        paddingInline: { default: "16px", [site.narrow]: "12px" },
    },
    gmBrand: {
        alignItems: "center",
        color: gmail.muted,
        display: { default: "flex", [site.narrow]: "none" },
        fontSize: "22px",
        gap: "14px",
        width: "214px",
    },
    gmSearch: {
        alignItems: "center",
        backgroundColor: { default: gmail.field, [site.narrow]: gmail.surface },
        borderRadius: "999px",
        color: gmail.muted,
        display: "flex",
        flexGrow: 1,
        fontSize: "15px",
        gap: "14px",
        height: "46px",
        paddingInline: "18px",
    },
    gmFace: {
        alignItems: "center",
        backgroundColor: palette.teal,
        borderRadius: "50%",
        color: "white",
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "14px",
        fontWeight: 500,
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },
    gmFrame: {
        display: "grid",
        gridTemplateColumns: { default: "240px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        minHeight: 0,
        paddingInlineEnd: { default: "16px", [site.narrow]: "0" },
    },
    gmSide: {
        alignContent: "start",
        display: { default: "grid", [site.narrow]: "none" },
        gap: "2px",
        paddingInlineEnd: "16px",
    },
    gmCompose: {
        alignItems: "center",
        backgroundColor: gmail.compose,
        borderRadius: "16px",
        display: "flex",
        fontWeight: 500,
        gap: "12px",
        height: "56px",
        justifySelf: "start",
        marginBlockEnd: "16px",
        marginInlineStart: "8px",
        paddingInlineEnd: "24px",
        paddingInlineStart: "20px",
    },
    gmFolder: {
        alignItems: "center",
        borderEndEndRadius: "999px",
        borderStartEndRadius: "999px",
        display: "flex",
        height: "32px",
        justifyContent: "space-between",
        paddingInlineEnd: "12px",
        paddingInlineStart: "26px",
    },
    gmFolderOn: {
        backgroundColor: gmail.selected,
        fontWeight: 700,
    },
    gmList: {
        alignContent: "start",
        backgroundColor: gmail.surface,
        borderStartEndRadius: { default: "16px", [site.narrow]: "0" },
        borderStartStartRadius: { default: "16px", [site.narrow]: "0" },
        display: "grid",
        listStyle: "none",
        margin: 0,
        overflow: "hidden",
        padding: 0,
    },
    gmRow: {
        alignItems: "center",
        borderBottomColor: gmail.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: { default: "1px", [site.narrow]: "0" },
        columnGap: { default: "16px", [site.narrow]: "12px" },
        display: "grid",
        gridTemplateAreas: {
            default: "'sender line time'",
            [site.narrow]: "'avatar sender time' 'avatar line line'",
        },
        gridTemplateColumns: {
            default: "168px minmax(0, 1fr) auto",
            [site.narrow]: "40px minmax(0, 1fr) auto",
        },
        minHeight: { default: "40px", [site.narrow]: "72px" },
        paddingInline: { default: "16px", [site.narrow]: "16px" },
        whiteSpace: "nowrap",
    },
    gmRowRead: {
        backgroundColor: { default: gmail.read, [site.narrow]: gmail.surface },
    },
    gmAvatar: {
        alignItems: "center",
        borderRadius: "50%",
        color: "white",
        display: { default: "none", [site.narrow]: "inline-flex" },
        fontSize: "16px",
        gridArea: "avatar",
        height: "40px",
        justifyContent: "center",
        width: "40px",
    },
    gmSender: {
        gridArea: "sender",
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    gmLine: {
        gridArea: "line",
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    gmUnread: {
        fontWeight: 700,
    },
    gmPreview: {
        color: gmail.muted,
    },
    gmTime: {
        fontSize: "12px",
        gridArea: "time",
    },
    gmWindow: {
        backgroundColor: gmail.surface,
        borderStartEndRadius: "8px",
        borderStartStartRadius: "8px",
        bottom: 0,
        boxShadow: "0 8px 24px color-mix(in srgb, black 18%, transparent)",
        display: "grid",
        insetInlineEnd: { default: "24px", [site.narrow]: "0" },
        insetInlineStart: { default: "auto", [site.narrow]: "0" },
        position: "absolute",
        width: { default: "560px", [site.narrow]: "auto" },
    },
    gmWindowHead: {
        backgroundColor: gmail.header,
        borderStartEndRadius: "8px",
        borderStartStartRadius: "8px",
        fontWeight: 500,
        paddingBlock: "10px",
        paddingInline: "16px",
    },
    gmField: {
        borderBottomColor: gmail.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "8px",
        marginInline: "16px",
        overflow: "hidden",
        paddingBlock: "8px",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    gmWarning: {
        color: gmail.warning,
        fontSize: "13px",
        paddingBlock: "12px",
        paddingInline: "16px",
    },
    gmSend: {
        backgroundColor: gmail.blue,
        borderRadius: "999px",
        color: "white",
        fontWeight: 500,
        justifySelf: "start",
        marginBlockEnd: 0,
        marginInlineStart: 0,
        opacity: 0.5,
        paddingBlock: "8px",
        paddingInline: "22px",
    },
    gmBody: {
        fontSize: "14px",
        lineHeight: 1.5,
        margin: 0,
        minHeight: { default: "120px", [site.narrow]: "0" },
        paddingBlock: "12px",
        paddingInline: "16px",
    },
    gmTools: {
        alignItems: "center",
        color: gmail.muted,
        display: "flex",
        gap: "16px",
        paddingBlockEnd: "14px",
        paddingInline: "16px",
    },
});
