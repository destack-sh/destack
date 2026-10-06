import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/icon";
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
    name: "no-notebooks",
    description: "a notebook list with no notebooks yet and the action that creates the first",
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
