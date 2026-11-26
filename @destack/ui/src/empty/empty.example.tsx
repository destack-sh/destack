import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import { Button } from "../button/index.ts";
import {
    Empty,
    EmptyContent,
    EmptyDescription,
    EmptyHeader,
    EmptyMedia,
    EmptyTitle,
} from "./empty.tsx";

/** A notebook list with no notebooks yet and the action that creates the first. */
export const emptyNoNotebooks = defineExample({
    of: Empty,
    name: "noNotebooks",
    description: t`A notebook list with no notebooks yet and the action that creates the first.`,
    render: () => (
        <Empty>
            <EmptyHeader>
                <EmptyMedia variant="icon">
                    <Icon name="notebook" />
                </EmptyMedia>
                <EmptyTitle>No notebooks yet</EmptyTitle>
                <EmptyDescription>Notebooks keep related notes together.</EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
                <Button>Create a notebook</Button>
            </EmptyContent>
        </Empty>
    ),
});

/** A search that found nothing, with the action that clears it. */
export const emptySearchNoResults = defineExample({
    of: Empty,
    name: "searchNoResults",
    description: t`A search that found nothing, with the action that clears it.`,
    render: () => (
        <Empty>
            <EmptyHeader>
                <EmptyMedia variant="icon">
                    <Icon name="magnifying-glass" />
                </EmptyMedia>
                <EmptyTitle>No notes match</EmptyTitle>
                <EmptyDescription>Try another word, or clear the search.</EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
                <Button variant="outline">Clear the search</Button>
            </EmptyContent>
        </Empty>
    ),
});
