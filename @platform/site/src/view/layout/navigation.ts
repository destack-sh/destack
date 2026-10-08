import type { IconBodies } from "@destack/icon";
import bookOpen from "@destack/icon/phosphor/book-open";
import house from "@destack/icon/phosphor/house";
import discordLogo from "@destack/icon/phosphor/discord-logo";
import envelopeSimple from "@destack/icon/phosphor/envelope-simple";
import githubLogo from "@destack/icon/phosphor/github-logo";
import linkedinLogo from "@destack/icon/phosphor/linkedin-logo";
import megaphone from "@destack/icon/phosphor/megaphone";
import packageIcon from "@destack/icon/phosphor/package";
import xLogo from "@destack/icon/phosphor/x-logo";
import type { Accelerator } from "@destack/view/palette";

/** A site destination, with the key combination that opens it. */
export interface Destination {
    /** The destination's address. */
    readonly href: string;
    /** The destination's label. */
    readonly label: string;
    /** The key combination that opens the destination. */
    readonly keybinding: Accelerator;
    /** The destination's icon. */
    readonly icon: IconBodies;
}

/** A site destination that opens once it is live. */
export type PendingDestination = Pick<Destination, "label" | "icon">;

// TODO #Incomplete: open packages once the registry is live
/** The site destinations that open once they are live, ahead of the primary ones. */
export const pendingDestinations: readonly PendingDestination[] = [
    { label: "Packages", icon: packageIcon },
];

/** The home page, which the brand links to. */
export const home: Destination = { href: "/", label: "Home", keybinding: "alt+h", icon: house };

/** The primary site destinations. */
export const primaryDestinations: readonly Destination[] = [
    { href: "/docs/", label: "Documentation", keybinding: "alt+d", icon: bookOpen },
    { href: "/blog/", label: "Blog", keybinding: "alt+b", icon: megaphone },
];

/** The Destack community destinations. */
export const socialDestinations: readonly Destination[] = [
    { href: "https://x.com/destacksh", label: "X", keybinding: "alt+x", icon: xLogo },
    {
        href: "https://www.linkedin.com/company/76992016",
        label: "LinkedIn",
        keybinding: "alt+l",
        icon: linkedinLogo,
    },
    {
        href: "https://github.com/destack-sh/destack",
        label: "GitHub",
        keybinding: "alt+g",
        icon: githubLogo,
    },
    {
        href: "https://discord.gg/xUFQ45TWYd",
        label: "Discord",
        keybinding: "alt+c",
        icon: discordLogo,
    },
    {
        href: "mailto:florian@symbol.industries",
        label: "Email",
        keybinding: "alt+e",
        icon: envelopeSimple,
    },
];

/** Every site destination with a key combination. */
export const destinations: readonly Destination[] = [
    home,
    ...primaryDestinations,
    ...socialDestinations,
];

/** Report whether a destination leaves the site, opening in a new tab or another application. */
export function isExternal(destination: Pick<Destination, "href">): boolean {
    return !destination.href.startsWith("/");
}
