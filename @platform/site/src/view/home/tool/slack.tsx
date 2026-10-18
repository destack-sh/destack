import * as style from "@destack/style";

import { messages } from "../verb/corner";
import { Glyph } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { palette } from "../../palette.stylex";
import { slack } from "./slack.stylex";

/** Draw the launch channel as a team chat seen through a guest invite. */
export function Slack() {
    return (
        <div data-component="SlackPage" {...style.attrs(styles.slack)}>
            <div {...style.attrs(styles.slackTop)}>
                <span {...style.attrs(styles.slackSearch)}>
                    <Glyph name="search" size={14} />
                    Search Symbol
                </span>
            </div>
            <div {...style.attrs(styles.slackFrame)}>
                <nav {...style.attrs(styles.slackRail)}>
                    <span {...style.attrs(styles.slackTeamIcon)}>L</span>
                    <Glyph name="inbox" size={18} />
                    <Glyph name="mail" size={18} />
                    <Glyph name="grid" size={18} />
                </nav>
                <nav {...style.attrs(styles.slackSide)}>
                    <b {...style.attrs(styles.slackTeam)}>
                        Symbol <Glyph name="caret" size={12} />
                    </b>
                    <span {...style.attrs(styles.slackGroup)}>▾ Channels</span>
                    {["general", "launch", "design", "pricing", "random"].map((name) => (
                        <span
                            {...style.attrs(
                                styles.slackChannel,
                                name === "launch" && styles.slackOn,
                            )}
                        >
                            # {name}
                        </span>
                    ))}
                    <span {...style.attrs(styles.slackGroup)}>▾ Apps</span>
                    <span {...style.attrs(styles.slackChannel)}>
                        <span {...style.attrs(styles.slackBot)}>A</span>
                        Agent
                    </span>
                </nav>
                <div {...style.attrs(styles.slackMain)}>
                    <span {...style.attrs(styles.slackHeader)}>
                        <b># launch</b>
                        <span {...style.attrs(styles.slackMembers)}>
                            <span
                                {...style.attrs(styles.slackFace)}
                                style={{ "background-color": "#2f7d8c" }}
                            >
                                F
                            </span>
                            <span
                                {...style.attrs(styles.slackFace)}
                                style={{ "background-color": "#6b5ca5" }}
                            >
                                A
                            </span>
                            4
                        </span>
                    </span>
                    <span {...style.attrs(styles.slackTabs)}>
                        <span {...style.attrs(styles.slackTab, styles.slackTabOn)}>Messages</span>
                        <span {...style.attrs(styles.slackTab)}>Canvas</span>
                        <span {...style.attrs(styles.slackTab)}>Files</span>
                    </span>
                    <ul {...style.attrs(styles.slackMessages)}>
                        {messages.map(([author, time, text], index) => (
                            <li {...style.attrs(styles.slackMessage)}>
                                <span
                                    style={{
                                        "background-color":
                                            author === "Agent" ? "#6b5ca5" : "#2f7d8c",
                                    }}
                                    {...style.attrs(styles.slackAvatar)}
                                >
                                    {author === "Agent" ? "A" : "F"}
                                </span>
                                <span {...style.attrs(styles.slackBody)}>
                                    <span>
                                        <b>{author === "Agent" ? "Agent" : "Florian"}</b>
                                        {author === "Agent" ? (
                                            <span {...style.attrs(styles.slackApp)}>APP</span>
                                        ) : undefined}
                                        <span {...style.attrs(styles.slackMuted)}> {time}</span>
                                    </span>
                                    {text}
                                    {index === 1 ? (
                                        <span {...style.attrs(styles.slackReact)}>
                                            <span {...style.attrs(styles.slackChip)}>👍 2</span>
                                            <span {...style.attrs(styles.slackChip)}>🚀 1</span>
                                        </span>
                                    ) : undefined}
                                    {index === 2 ? (
                                        <span {...style.attrs(styles.slackThread)}>
                                            3 replies · Last reply today at 09:52
                                        </span>
                                    ) : undefined}
                                </span>
                            </li>
                        ))}
                    </ul>
                    <span {...style.attrs(styles.slackComposer)}>
                        <span {...style.attrs(styles.slackMuted)}>Message #launch</span>
                        <span {...style.attrs(styles.slackTools)}>
                            <span>+</span>
                            <span>Aa</span>
                            <span>☺</span>
                            <span>@</span>
                        </span>
                    </span>
                    <p {...style.attrs(styles.slackGuest)}>
                        You're a guest in Symbol. Ask an admin to see more channels.
                    </p>
                </div>
            </div>
        </div>
    );
}

/** The slack page styles. */
const styles = style.create({
    slack: {
        backgroundColor: slack.surface,
        color: slack.ink,
        display: "grid",
        fontFamily: slack.face,
        fontSize: "15px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "40px minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
    },
    slackTop: {
        alignItems: "center",
        backgroundColor: slack.aubergine,
        display: "flex",
        justifyContent: "center",
    },
    slackSearch: {
        alignItems: "center",
        backgroundColor: `color-mix(in srgb, white 15%, transparent)`,
        borderRadius: "6px",
        color: `color-mix(in srgb, white 80%, transparent)`,
        display: "flex",
        fontSize: "13px",
        gap: "8px",
        paddingBlock: "5px",
        paddingInline: "12px",
        width: "48%",
    },
    slackFrame: {
        display: "grid",
        gridTemplateColumns: {
            default: "64px 230px minmax(0, 1fr)",
            [site.narrow]: "minmax(0, 1fr)",
        },
        minHeight: 0,
    },
    slackRail: {
        alignContent: "start",
        backgroundColor: slack.aubergine,
        color: `color-mix(in srgb, white 70%, transparent)`,
        display: { default: "grid", [site.narrow]: "none" },
        gap: "22px",
        justifyItems: "center",
        paddingTop: "12px",
    },
    slackTeamIcon: {
        alignItems: "center",
        backgroundColor: "white",
        borderRadius: "8px",
        color: slack.aubergine,
        display: "flex",
        fontWeight: 800,
        height: "36px",
        justifyContent: "center",
        width: "36px",
    },
    slackSide: {
        alignContent: "start",
        backgroundColor: slack.sidebar,
        color: `color-mix(in srgb, white 72%, transparent)`,
        display: { default: "grid", [site.narrow]: "none" },
        gap: "1px",
        paddingBlock: "14px",
        paddingInline: "10px",
    },
    slackTeam: {
        alignItems: "center",
        color: "white",
        display: "flex",
        fontSize: "18px",
        gap: "6px",
        marginBottom: "12px",
        paddingInline: "8px",
    },
    slackGroup: {
        fontSize: "14px",
        marginTop: "10px",
        paddingBlock: "4px",
        paddingInline: "8px",
    },
    slackChannel: {
        alignItems: "center",
        borderRadius: "6px",
        display: "flex",
        gap: "8px",
        paddingBlock: "4px",
        paddingInline: "14px",
    },
    slackOn: {
        backgroundColor: slack.link,
        color: "white",
        fontWeight: 700,
    },
    slackBot: {
        alignItems: "center",
        backgroundColor: palette.violet,
        borderRadius: "4px",
        color: "white",
        display: "flex",
        fontSize: "11px",
        fontWeight: 700,
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    slackMain: {
        display: "grid",
        gridTemplateRows: "auto auto minmax(0, 1fr) auto auto",
        minWidth: 0,
    },
    slackHeader: {
        alignItems: "center",
        display: "flex",
        fontSize: "18px",
        justifyContent: "space-between",
        paddingBlockStart: "12px",
        paddingBlockEnd: "6px",
        paddingInline: { default: "20px", [site.narrow]: "16px" },
    },
    slackMembers: {
        alignItems: "center",
        borderColor: slack.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        fontSize: "13px",
        gap: "4px",
        paddingBlock: "2px",
        paddingInlineStart: "4px",
        paddingInlineEnd: "8px",
    },
    slackFace: {
        alignItems: "center",
        borderRadius: "4px",
        color: "white",
        display: "flex",
        fontSize: "10px",
        fontWeight: 700,
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    slackTabs: {
        borderBottomColor: slack.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "16px",
        paddingInline: "20px",
    },
    slackTab: {
        color: slack.muted,
        fontSize: "13px",
        paddingBlock: "8px",
    },
    slackTabOn: {
        boxShadow: `inset 0 -2px 0 ${slack.ink}`,
        color: slack.ink,
        fontWeight: 700,
    },
    slackMuted: {
        color: slack.muted,
        fontSize: "13px",
    },
    slackMessages: {
        alignContent: "start",
        display: "grid",
        gap: "16px",
        listStyle: "none",
        margin: 0,
        overflow: "hidden",
        paddingBlock: "16px",
        paddingInline: "20px",
    },
    slackMessage: {
        display: "flex",
        gap: "10px",
    },
    slackAvatar: {
        alignItems: "center",
        borderRadius: "6px",
        color: "white",
        display: "flex",
        flexShrink: 0,
        fontWeight: 700,
        height: "36px",
        justifyContent: "center",
        width: "36px",
    },
    slackBody: {
        display: "grid",
        gap: "3px",
        lineHeight: 1.45,
    },
    slackApp: {
        backgroundColor: slack.rule,
        borderRadius: "3px",
        color: slack.muted,
        fontSize: "10px",
        fontWeight: 700,
        marginLeft: "6px",
        paddingInline: "4px",
    },
    slackReact: {
        display: "flex",
        gap: "6px",
        marginTop: "2px",
    },
    slackChip: {
        backgroundColor: slack.mention,
        borderColor: slack.highlight,
        borderRadius: "12px",
        borderStyle: "solid",
        borderWidth: "1px",
        fontSize: "12px",
        paddingInline: "8px",
    },
    slackThread: {
        color: slack.link,
        fontSize: "13px",
        fontWeight: 700,
    },
    slackComposer: {
        borderColor: slack.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        gap: "10px",
        marginBlockStart: "0",
        marginBlockEnd: "10px",
        marginInline: "20px",
        paddingBlock: "10px",
        paddingInline: "12px",
    },
    slackTools: {
        color: slack.muted,
        display: "flex",
        fontSize: "14px",
        gap: "16px",
    },
    slackGuest: {
        backgroundColor: slack.notice,
        color: slack.noticeInk,
        fontSize: "13px",
        margin: 0,
        paddingBlock: "8px",
        paddingInline: "20px",
    },
});
