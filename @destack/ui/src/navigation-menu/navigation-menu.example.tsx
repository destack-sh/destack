import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, weight } from "@destack/theme/tokens.stylex";
import type { JSX } from "@destack/view";
import {
    NavigationMenu,
    NavigationMenuContent,
    NavigationMenuIndicator,
    NavigationMenuItem,
    NavigationMenuLink,
    NavigationMenuList,
    NavigationMenuTrigger,
    navigationMenuTriggerStyle,
} from "./navigation-menu.tsx";

/** The styles of the products panel's links. */
const styles = style.create({
    link: { width: "16rem" },
    title: { fontWeight: weight.medium },
    detail: { color: color.mutedForeground },
});

/** Render the links of a products panel, each with what the product is for. */
function Products(): JSX.Element {
    return (
        <>
            <NavigationMenuLink href="/notes" xstyle={styles.link}>
                <span {...style.attrs(text.sm, styles.title)}>Notes</span>
                <span {...style.attrs(text.sm, styles.detail)}>
                    Write things down and find them again
                </span>
            </NavigationMenuLink>
            <NavigationMenuLink href="/tasks" xstyle={styles.link}>
                <span {...style.attrs(text.sm, styles.title)}>Tasks</span>
                <span {...style.attrs(text.sm, styles.detail)}>Plan the work with your team</span>
            </NavigationMenuLink>
        </>
    );
}

/** A site's main navigation with a products panel, on its pricing page. */
export const navigationMenuSiteNavigation = defineExample({
    of: NavigationMenu,
    name: "siteNavigation",
    description: t`A site's main navigation with a products panel, on its pricing page.`,
    properties: {
        "aria-label": "Main",
    },
    render: (properties) => (
        <NavigationMenu {...properties}>
            <NavigationMenuList>
                <NavigationMenuItem>
                    <NavigationMenuTrigger>Products</NavigationMenuTrigger>
                    <NavigationMenuContent>
                        <Products />
                    </NavigationMenuContent>
                </NavigationMenuItem>
                <NavigationMenuItem>
                    <NavigationMenuLink
                        href="/pricing"
                        active
                        xstyle={navigationMenuTriggerStyle()}
                    >
                        Pricing
                    </NavigationMenuLink>
                </NavigationMenuItem>
            </NavigationMenuList>
        </NavigationMenu>
    ),
});

/** A site's main navigation with a products panel and a company panel. */
export const navigationMenuTwoPanels = defineExample({
    of: NavigationMenu,
    name: "twoPanels",
    description: t`A site's main navigation with a products panel and a company panel.`,
    properties: {
        "aria-label": "Main",
    },
    render: (properties) => (
        <NavigationMenu {...properties}>
            <NavigationMenuList>
                <NavigationMenuItem>
                    <NavigationMenuTrigger>Products</NavigationMenuTrigger>
                    <NavigationMenuContent>
                        <Products />
                    </NavigationMenuContent>
                </NavigationMenuItem>
                <NavigationMenuItem>
                    <NavigationMenuTrigger>Company</NavigationMenuTrigger>
                    <NavigationMenuContent>
                        <NavigationMenuLink href="/about">About</NavigationMenuLink>
                    </NavigationMenuContent>
                </NavigationMenuItem>
                <NavigationMenuIndicator />
            </NavigationMenuList>
        </NavigationMenu>
    ),
});

/** The site's main navigation with its products panel open. */
export const navigationMenuProductsOpen = defineExample({
    of: NavigationMenu,
    name: "productsOpen",
    description: t`The site's main navigation with its products panel open.`,
    properties: {
        "aria-label": "Main",
        defaultValue: "products",
    },
    render: (properties) => (
        <NavigationMenu {...properties}>
            <NavigationMenuList>
                <NavigationMenuItem value="products">
                    <NavigationMenuTrigger>Products</NavigationMenuTrigger>
                    <NavigationMenuContent>
                        <Products />
                    </NavigationMenuContent>
                </NavigationMenuItem>
                <NavigationMenuItem value="company">
                    <NavigationMenuTrigger>Company</NavigationMenuTrigger>
                    <NavigationMenuContent>
                        <NavigationMenuLink href="/about">About</NavigationMenuLink>
                    </NavigationMenuContent>
                </NavigationMenuItem>
                <NavigationMenuIndicator />
            </NavigationMenuList>
        </NavigationMenu>
    ),
});

/** A site's company link, set on its own. */
export const navigationMenuLinkDefault = defineExample({
    of: NavigationMenuLink,
    name: "default",
    description: t`A site's company link, set on its own.`,
    render: (properties) => (
        <NavigationMenu aria-label="Site">
            <NavigationMenuList>
                <NavigationMenuItem>
                    <NavigationMenuLink href="/about" {...properties}>
                        About
                    </NavigationMenuLink>
                </NavigationMenuItem>
            </NavigationMenuList>
        </NavigationMenu>
    ),
});
