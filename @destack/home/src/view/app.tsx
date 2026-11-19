import * as style from "@destack/style";
import type { SwatchName } from "@destack/theme";
import { color, radius, space, swatch } from "@destack/theme/tokens.stylex";
import { Item, ItemContent, ItemDescription, ItemMedia, ItemTitle } from "@destack/ui/item";
import type { JSX } from "@destack/view";

/** The styles of an app's mark, its link and a grid of links. */
const styles = style.create({
    grid: {
        display: "grid",
        gap: space[2],
        gridTemplateColumns: "repeat(auto-fill, minmax(14rem, 1fr))",
    },
    link: {
        color: "inherit",
        textDecoration: "none",
    },
    mark: {
        alignItems: "center",
        borderRadius: radius[2],
        backgroundColor: color.primary,
        color: color.primaryForeground,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "0.75rem",
        fontWeight: 700,
        blockSize: "1.25rem",
        inlineSize: "1.25rem",
        justifyContent: "center",
    },
    swatch: (fill: string, label: string) => ({ backgroundColor: fill, color: label }),
});

/** Mark an app by its title's first letter, in its package's swatch. */
export function AppMark(properties: {
    /** The app's title. */
    title: string;
    /** The package's swatch, the primary color for none. */
    color: SwatchName | undefined;
}) {
    // fill the mark in the swatch's solid color with its label color on top
    const filled = () =>
        properties.color === undefined
            ? undefined
            : styles.swatch(swatch[`${properties.color}Solid`], swatch[`${properties.color}Label`]);

    return (
        <span aria-hidden="true" {...style.attrs(styles.mark, filled())}>
            {properties.title.slice(0, 1).toUpperCase()}
        </span>
    );
}

/** Lay out app links in a grid of equal columns. */
export function AppGrid(properties: {
    /** The links. */
    children: JSX.Element;
}) {
    return <div {...style.attrs(styles.grid)}>{properties.children}</div>;
}

/** Link an app's view by its mark, its title and its installation's name. */
export function AppLink(properties: {
    /** Where the link opens the app's view. */
    href: string;
    /** The app's title. */
    title: string;
    /** The package's swatch, the primary color for none. */
    color: SwatchName | undefined;
    /** The installation's name. */
    name: string;
}) {
    return (
        <a href={properties.href} {...style.attrs(styles.link)}>
            <Item variant="outline" size="sm">
                <ItemMedia>
                    <AppMark title={properties.title} color={properties.color} />
                </ItemMedia>
                <ItemContent>
                    <ItemTitle>{properties.title}</ItemTitle>
                    <ItemDescription>{properties.name}</ItemDescription>
                </ItemContent>
            </Item>
        </a>
    );
}
