import { Icon } from "@destack/icon";
import type { JSX } from "@solidjs/web";
import { Button } from "../button/index.ts";
import {
    Empty,
    EmptyContent,
    EmptyDescription,
    EmptyHeader,
    EmptyMedia,
    EmptyTitle,
} from "./empty.tsx";

/** Show a notebook list with no notebooks yet and the action that creates the first. */
export function EmptyExample(): JSX.Element {
    return (
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
    );
}
