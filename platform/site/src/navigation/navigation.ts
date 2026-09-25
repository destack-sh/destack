import discordIcon from "./icons/discord.svg?raw";
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
};

/** The primary internal site destinations. */
export const primaryLinks: readonly NavigationLink[] = [
    { href: "/docs/", label: "Documentation", shortcut: "d" },
    { href: "/blog/", label: "Blog", shortcut: "b" },
];

/** The external Destack community destinations. */
export const socialLinks: readonly (NavigationLink & { icon: string })[] = [
    { href: "https://discord.gg/xUFQ45TWYd", label: "Discord", shortcut: "c", icon: discordIcon },
    { href: "https://x.com/destack", label: "X", shortcut: "x", icon: xIcon },
    {
        href: "https://github.com/destack-sh/destack",
        label: "GitHub",
        shortcut: "g",
        icon: githubIcon,
    },
];

/** Every persistent site destination. */
export const navigationLinks: readonly NavigationLink[] = [...primaryLinks, ...socialLinks];
