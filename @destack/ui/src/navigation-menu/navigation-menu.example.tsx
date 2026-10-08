import { defineExample } from "@destack/package/declare";
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

/** A site's main navigation with a products panel, on its pricing page. */
export const navigationMenuSiteNavigation = defineExample({
    of: NavigationMenu,
    name: "site-navigation",
    description: "a site's main navigation with a products panel, on its pricing page",
    render: () => (
        <NavigationMenu aria-label="Main">
            <NavigationMenuList>
                <NavigationMenuItem>
                    <NavigationMenuTrigger>Products</NavigationMenuTrigger>
                    <NavigationMenuContent>
                        <NavigationMenuLink href="/notes">Notes</NavigationMenuLink>
                        <NavigationMenuLink href="/tasks">Tasks</NavigationMenuLink>
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
    name: "two-panels",
    description: "a site's main navigation with a products panel and a company panel",
    render: () => (
        <NavigationMenu aria-label="Main">
            <NavigationMenuList>
                <NavigationMenuItem>
                    <NavigationMenuTrigger>Products</NavigationMenuTrigger>
                    <NavigationMenuContent>
                        <NavigationMenuLink href="/notes">Notes</NavigationMenuLink>
                        <NavigationMenuLink href="/tasks">Tasks</NavigationMenuLink>
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
    name: "products-open",
    description: "the site's main navigation with its products panel open",
    render: () => (
        <NavigationMenu aria-label="Main" defaultValue="products">
            <NavigationMenuList>
                <NavigationMenuItem value="products">
                    <NavigationMenuTrigger>Products</NavigationMenuTrigger>
                    <NavigationMenuContent>
                        <NavigationMenuLink href="/notes">Notes</NavigationMenuLink>
                        <NavigationMenuLink href="/tasks">Tasks</NavigationMenuLink>
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
