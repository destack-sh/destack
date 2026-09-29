import agentsIcon from "./icons/agents.svg?raw";
import appsIcon from "./icons/apps.svg?raw";
import blogIcon from "./icons/blog.svg?raw";
import discordIcon from "./icons/discord.svg?raw";
import documentationIcon from "./icons/documentation.svg?raw";
import emailIcon from "./icons/email.svg?raw";
import githubIcon from "./icons/github.svg?raw";
import xIcon from "./icons/x.svg?raw";

/** One persistent site destination and its keyboard mnemonic. */
export type NavigationLink = {
    /** The destination URL. */
    href: string;

    /** The visible navigation label. */
    label: string;

    /** The global keyboard mnemonic. */
    shortcut: string;

    /** The icon's SVG source. */
    icon: string;
};

/** One site destination that opens once it is live. */
export type PendingLink = Pick<NavigationLink, "label" | "icon">;

// TODO #Incomplete: open apps and agents once the registry and accounts are live
/** The site destinations that open once they are live, ahead of the primary ones. */
export const pendingLinks: readonly PendingLink[] = [
    { label: "Apps", icon: appsIcon },
    { label: "Agents", icon: agentsIcon },
];

/** The primary internal site destinations. */
export const primaryLinks: readonly NavigationLink[] = [
    { href: "/docs/", label: "Documentation", shortcut: "d", icon: documentationIcon },
    { href: "/blog/", label: "Blog", shortcut: "b", icon: blogIcon },
];

/** The external Destack community destinations. */
export const socialLinks: readonly NavigationLink[] = [
    { href: "https://discord.gg/xUFQ45TWYd", label: "Discord", shortcut: "c", icon: discordIcon },
    { href: "https://x.com/destacksh", label: "X", shortcut: "x", icon: xIcon },
    {
        href: "https://github.com/destack-sh/destack",
        label: "GitHub",
        shortcut: "g",
        icon: githubIcon,
    },
    {
        href: "mailto:florian@symbol.industries",
        label: "Email",
        shortcut: "e",
        icon: emailIcon,
    },
];

/** Every persistent site destination. */
export const navigationLinks: readonly NavigationLink[] = [...primaryLinks, ...socialLinks];
