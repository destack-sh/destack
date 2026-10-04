import type { JSX } from "@solidjs/web";
import {
    NavigationMenu,
    NavigationMenuContent,
    NavigationMenuItem,
    NavigationMenuLink,
    NavigationMenuList,
    NavigationMenuTrigger,
    navigationMenuTriggerStyle,
} from "./navigation-menu.tsx";

/** Show a site's main navigation with a products panel. */
export function NavigationMenuExample(): JSX.Element {
    return (
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
                    <NavigationMenuLink href="/pricing" style={navigationMenuTriggerStyle()}>
                        Pricing
                    </NavigationMenuLink>
                </NavigationMenuItem>
            </NavigationMenuList>
        </NavigationMenu>
    );
}
