/// One persistent site destination and its keyboard mnemonic.
export type NavigationLink = {
    /// The destination URL.
    href: string;

    /// The visible navigation label.
    label: string;

    /// The global keyboard mnemonic.
    shortcut: string;
};

/// The primary internal site destinations.
export const primaryLinks: readonly NavigationLink[] = [
    { href: "/docs/", label: "docs", shortcut: "d" },
    { href: "/blog/", label: "blog", shortcut: "b" },
];

/// The external Destack community destinations.
export const socialLinks: readonly NavigationLink[] = [
    { href: "https://discord.gg/xUFQ45TWYd", label: "discord", shortcut: "c" },
    { href: "https://x.com/destack", label: "x", shortcut: "x" },
    { href: "https://github.com/destack-sh/destack", label: "github", shortcut: "g" },
];

/// Every persistent site destination.
export const navigationLinks: readonly NavigationLink[] = [...primaryLinks, ...socialLinks];
