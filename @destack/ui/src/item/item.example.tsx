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
    itemVariants,
} from "./item.tsx";

/** The styles of a group of outlined items, set apart. */
const styles = style.create({
    apart: { gap: space[2] },
});

/** A notebook's recent notes, one archivable and one opened by a link. */
export const itemRecentNotes = defineExample({
    of: Item,
    name: "recent-notes",
    description: "a notebook's recent notes, one archivable and one opened by a link",
    render: () => (
        <ItemGroup aria-label="Recent notes">
            <Item>
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
                <a href="/notes/trip" {...style.attrs(itemVariants({ size: "sm" }))}>
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
    name: "recent-notes-outlined",
    description: "recent notes as outlined items",
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
                    {...style.attrs(itemVariants({ variant: "outline", size: "sm" }))}
                >
                    <ItemContent>
                        <ItemTitle>Trip to Lisbon</ItemTitle>
                    </ItemContent>
                </a>
            </div>
        </ItemGroup>
    ),
});
