import * as style from "@destack/style";

import { joiners, perks } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { replit } from "./replit.stylex";

/** Draw your waitlist as an app built by prompt and hosted on the builder's own domain. */
export function Replit() {
    return (
        <div data-component="ReplitPage" {...style.attrs(styles.replit)}>
            <nav {...style.attrs(styles.replitNav)}>
                <b {...style.attrs(styles.replitBrand)}>
                    <span {...style.attrs(styles.replitLogo)}>✦</span>
                    Destack
                </b>
                <span {...style.attrs(styles.replitLinks)}>
                    <span>Features</span>
                    <span>Pricing</span>
                    <span>FAQ</span>
                </span>
            </nav>
            <div {...style.attrs(styles.replitHero)}>
                <span {...style.attrs(styles.replitPill)}>🚀 Launching Friday</span>
                <b {...style.attrs(styles.replitTitle)}>
                    The software{" "}
                    <span {...style.attrs(styles.replitGlow)}>you'll actually own</span>
                </b>
                <span {...style.attrs(styles.replitLine)}>
                    Destack opens on Friday. Leave your email and we'll send you an invite.
                </span>
                <span {...style.attrs(styles.replitForm)}>
                    <span {...style.attrs(styles.replitInput)}>you@company.com</span>
                    <span {...style.attrs(styles.replitButton)}>Join waitlist →</span>
                </span>
                <span {...style.attrs(styles.replitProof)}>
                    <span {...style.attrs(styles.replitFaces)}>
                        {joiners.map(([initial, tint]) => (
                            <span
                                style={{ "background-color": tint }}
                                {...style.attrs(styles.replitFace)}
                            >
                                {initial}
                            </span>
                        ))}
                    </span>
                    Join 1,840 others on the list
                </span>
            </div>
            <ul {...style.attrs(styles.replitPerks)}>
                {perks.map(([title, line]) => (
                    <li {...style.attrs(styles.replitPerk)}>
                        <b>{title}</b>
                        <span {...style.attrs(styles.replitPerkLine)}>{line}</span>
                    </li>
                ))}
            </ul>
            <span data-component="VendorBadge" {...style.attrs(styles.replitBadge)}>
                <Favicon icon="replit" tint="#f26207" size={14} />
                Made with Replit
            </span>
        </div>
    );
}

/** The replit page styles. */
const styles = style.create({
    replit: {
        backgroundColor: replit.night,
        backgroundImage: `radial-gradient(ellipse 60% 45% at 50% 0%, color-mix(in srgb, ${replit.violet} 35%, transparent), transparent), radial-gradient(color-mix(in srgb, ${replit.snow} 7%, transparent) 1px, transparent 1px)`,
        backgroundSize: "auto, 22px 22px",
        color: replit.snow,
        display: "grid",
        alignContent: "start",
        fontFamily: `Inter, ${site.system}`,
        gap: "28px",
        gridTemplateColumns: "minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
        paddingBlockStart: "0",
        paddingBlockEnd: "48px",
        paddingInline: "48px",
        position: "relative",
    },
    replitNav: {
        alignItems: "center",
        display: "flex",
        height: "72px",
        justifyContent: "space-between",
    },
    replitBrand: {
        alignItems: "center",
        display: "flex",
        fontSize: "18px",
        gap: "8px",
    },
    replitLogo: {
        color: replit.lilac,
    },
    replitLinks: {
        color: `color-mix(in srgb, ${replit.snow} 60%, transparent)`,
        display: "flex",
        fontSize: "14px",
        gap: "28px",
    },
    replitHero: {
        display: "grid",
        gap: "14px",
        justifyItems: "center",
        marginBlock: "0",
        marginInline: "auto",
        maxWidth: "640px",
        textAlign: "center",
    },
    replitPill: {
        backgroundColor: `color-mix(in srgb, ${replit.snow} 6%, transparent)`,
        borderColor: `color-mix(in srgb, ${replit.snow} 14%, transparent)`,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        fontSize: "13px",
        paddingBlock: "6px",
        paddingInline: "14px",
    },
    replitGlow: {
        backgroundClip: "text",
        backgroundImage: `linear-gradient(90deg, ${replit.lilac}, ${replit.pink})`,
        color: "transparent",
    },
    replitProof: {
        alignItems: "center",
        color: `color-mix(in srgb, ${replit.snow} 60%, transparent)`,
        display: "flex",
        fontSize: "14px",
        gap: "10px",
    },
    replitFaces: {
        display: "flex",
    },
    replitFace: {
        alignItems: "center",
        borderColor: replit.night,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "2px",
        color: "white",
        display: "flex",
        fontSize: "11px",
        fontWeight: 700,
        height: "28px",
        justifyContent: "center",
        marginLeft: "-8px",
        width: "28px",
    },
    replitPerks: {
        display: "grid",
        gap: "16px",
        gridTemplateColumns: {
            default: "repeat(3, minmax(0, 1fr))",
            [site.narrow]: "minmax(0, 1fr)",
        },
        listStyle: "none",
        marginBlock: "0",
        marginInline: "auto",
        maxWidth: "820px",
        padding: 0,
        width: "100%",
    },
    replitPerk: {
        backgroundColor: `color-mix(in srgb, ${replit.snow} 4%, transparent)`,
        borderColor: `color-mix(in srgb, ${replit.snow} 10%, transparent)`,
        borderRadius: "14px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        fontSize: "15px",
        gap: "6px",
        padding: "18px",
    },
    replitPerkLine: {
        color: `color-mix(in srgb, ${replit.snow} 55%, transparent)`,
        fontSize: "13px",
    },
    replitTitle: {
        fontSize: "40px",
        fontWeight: 700,
        letterSpacing: "-0.03em",
        lineHeight: 1.1,
        maxWidth: "15ch",
    },
    replitLine: {
        color: `color-mix(in srgb, ${replit.snow} 70%, transparent)`,
        fontSize: "17px",
        lineHeight: 1.5,
    },
    replitForm: {
        display: "flex",
        gap: "10px",
        marginTop: "6px",
        maxWidth: "520px",
        width: "100%",
    },
    replitInput: {
        backgroundColor: `color-mix(in srgb, ${replit.night} 70%, transparent)`,
        borderColor: `color-mix(in srgb, ${replit.snow} 15%, transparent)`,
        borderRadius: "10px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: `color-mix(in srgb, ${replit.snow} 45%, transparent)`,
        flexGrow: 1,
        fontSize: "16px",
        paddingBlock: "14px",
        paddingInline: "16px",
        textAlign: "left",
    },
    replitButton: {
        backgroundImage: `linear-gradient(90deg, ${replit.violet}, ${replit.magenta})`,
        borderRadius: "10px",
        fontSize: "16px",
        fontWeight: 600,
        paddingBlock: "14px",
        paddingInline: "22px",
        whiteSpace: "nowrap",
    },
    replitBadge: {
        alignItems: "center",
        backgroundColor: replit.navy,
        borderColor: "transparent",
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        bottom: "20px",
        display: "flex",
        fontSize: "13px",
        fontWeight: 600,
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "12px",
        position: "absolute",
        right: "20px",
        color: "white",
    },
});
