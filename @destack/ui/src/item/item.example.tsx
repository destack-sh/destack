import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { Icon } from "@destack/ui/icon";
import { Button } from "../button/index.ts";
import {
    Item,
    ItemActions,
    ItemContent,
    ItemDescription,
    ItemGroup,
    ItemMedia,
    ItemSeparator,
    ItemTitle,
} from "./item.tsx";

/** The styles of a group of outlined items, set apart. */
const styles = style.create({
    apart: { gap: space[2] },
});

/** A notebook's recent notes, one archivable and one opened by a link. */
export const itemRecentNotes = defineExample({
    of: Item,
    name: "recentNotes",
    description: t`A notebook's recent notes, one archivable and one opened by a link.`,
    render: (properties) => (
        <ItemGroup aria-label="Recent notes">
            <Item {...properties}>
                <ItemMedia variant="icon">
                    <Icon name="notebook" />
                </ItemMedia>
                <ItemContent>
                    <ItemTitle>Groceries</ItemTitle>
                    <ItemDescription>Oat milk, lemons, rye bread</ItemDescription>
                </ItemContent>
                <ItemActions>
                    <Button variant="outline">Archive</Button>
                </ItemActions>
            </Item>
            <ItemSeparator />
            <div role="listitem">
                <a href="/notes/trip" {...style.attrs(Item.styles({ size: "sm" }))}>
                    <ItemContent>
                        <ItemTitle>Trip to Lisbon</ItemTitle>
                    </ItemContent>
                </a>
            </div>
        </ItemGroup>
    ),
});

/** Recent notes as outlined items. */
export const itemRecentNotesOutlined = defineExample({
    of: Item,
    name: "recentNotesOutlined",
    description: t`Recent notes as outlined items.`,
    properties: {
        variant: "outline",
    },
    render: (properties) => (
        <ItemGroup aria-label="Recent notes" xstyle={styles.apart}>
            <Item {...properties}>
                <ItemMedia variant="icon">
                    <Icon name="notebook" />
                </ItemMedia>
                <ItemContent>
                    <ItemTitle>Groceries</ItemTitle>
                    <ItemDescription>Oat milk, lemons, rye bread</ItemDescription>
                </ItemContent>
                <ItemActions>
                    <Button variant="outline">Archive</Button>
                </ItemActions>
            </Item>
            <div role="listitem">
                <a
                    href="/notes/trip"
                    {...style.attrs(Item.styles({ variant: "outline", size: "sm" }))}
                >
                    <ItemContent>
                        <ItemTitle>Trip to Lisbon</ItemTitle>
                    </ItemContent>
                </a>
            </div>
        </ItemGroup>
    ),
});

/** A note's row, its picture framed on its own. */
export const itemMediaDefault = defineExample({
    of: ItemMedia,
    name: "default",
    description: t`A note's row, its picture framed on its own.`,
    properties: { variant: "icon" },
    render: (properties) => (
        <Item variant="outline">
            <ItemMedia {...properties}>
                <Icon name="notebook" />
            </ItemMedia>
            <ItemContent>
                <ItemTitle>Trip to Lisbon</ItemTitle>
                <ItemDescription>Edited yesterday</ItemDescription>
            </ItemContent>
        </Item>
    ),
});
