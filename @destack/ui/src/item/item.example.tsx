import * as style from "@destack/style";
import { Icon } from "@destack/icon";
import type { JSX } from "@solidjs/web";
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
    itemStyle,
} from "./item.tsx";

/** Show a notebook's recent notes, one archivable and one opened by a link. */
export function ItemExample(): JSX.Element {
    return (
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
                    <Button variant="outline" size="sm">
                        Archive
                    </Button>
                </ItemActions>
            </Item>
            <ItemSeparator />
            <a href="/notes/trip" role="listitem" {...style.attrs(itemStyle({ size: "sm" }))}>
                <ItemContent>
                    <ItemTitle>Trip to Lisbon</ItemTitle>
                </ItemContent>
            </a>
        </ItemGroup>
    );
}
