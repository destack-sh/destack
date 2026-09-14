import { collections } from "../../content";

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
export const primaryLinks: readonly NavigationLink[] = collections.flatMap(
    (collection) =>
        collection.shortcut == undefined
            ? []
            : [
                  {
                      href: collection.route,
                      label: collection.title,
                      shortcut: collection.shortcut,
                  },
              ],
);

/// The external Destack community destinations.
export const socialLinks: readonly NavigationLink[] = [
    { href: "https://discord.gg/xUFQ45TWYd", label: "Discord", shortcut: "c" },
    { href: "https://x.com/destack", label: "Twitter", shortcut: "x" },
    {
        href: "https://github.com/destack-sh/destack",
        label: "GitHub",
        shortcut: "g",
    },
];

/// Every persistent site destination.
export const navigationLinks: readonly NavigationLink[] = [
    ...primaryLinks,
    ...socialLinks,
];
