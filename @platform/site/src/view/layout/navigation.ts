import type { IconBodies } from "@destack/icon";
import bookOpen from "@destack/icon/phosphor/book-open";
import house from "@destack/icon/phosphor/house";
import discordLogo from "@destack/icon/phosphor/discord-logo";
import githubLogo from "@destack/icon/phosphor/github-logo";
import linkedinLogo from "@destack/icon/phosphor/linkedin-logo";
import megaphone from "@destack/icon/phosphor/megaphone";
import packageIcon from "@destack/icon/phosphor/package";
import tag from "@destack/icon/phosphor/tag";
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
/** The site's menu in order, each destination live or opening once it is. */
export const menu: readonly (Destination | PendingDestination)[] = [
    { label: "Packages", icon: packageIcon },
    { href: "/pricing/", label: "Pricing", keybinding: "alt+p", icon: tag },
    { href: "/docs/", label: "Docs", keybinding: "alt+d", icon: bookOpen },
    { href: "/blog/", label: "Blog", keybinding: "alt+b", icon: megaphone },
];

/** The home page, which the brand links to. */
export const home: Destination = { href: "/", label: "Home", keybinding: "alt+h", icon: house };

/** The live destinations of the menu. */
export const primaryDestinations: readonly Destination[] = menu.filter(isLive);

/** Destack's repository on GitHub. */
export const github: Destination = {
    href: "https://github.com/destack-sh/destack",
    label: "GitHub",
    keybinding: "alt+g",
    icon: githubLogo,
};

/** The Destack community on Discord. */
export const discord: Destination = {
    href: "https://discord.gg/xUFQ45TWYd",
    label: "Discord",
    keybinding: "alt+c",
    icon: discordLogo,
};

/** Destack's places off the site: X, LinkedIn, the repository and the community. */
export const socialDestinations: readonly Destination[] = [
    { href: "https://x.com/destacksh", label: "X", keybinding: "alt+x", icon: xLogo },
    {
        href: "https://www.linkedin.com/company/76992016",
        label: "LinkedIn",
        keybinding: "alt+l",
        icon: linkedinLogo,
    },
    github,
    discord,
];

/** Every site destination with a key combination. */
export const destinations: readonly Destination[] = [
    home,
    ...primaryDestinations,
    ...socialDestinations,
];

/** Report whether a menu destination is live, with an address to open. */
export function isLive(destination: Destination | PendingDestination): destination is Destination {
    return "href" in destination;
}

/** Report whether a destination leaves the site, opening in a new tab or another application. */
export function isExternal(destination: Pick<Destination, "href">): boolean {
    return !destination.href.startsWith("/");
}
