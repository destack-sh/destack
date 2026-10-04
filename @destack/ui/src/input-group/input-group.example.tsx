import { Icon } from "@destack/icon";
import type { JSX } from "@solidjs/web";
import { Kbd } from "../kbd/index.ts";
import {
    InputGroup,
    InputGroupAddon,
    InputGroupButton,
    InputGroupInput,
    InputGroupText,
} from "./input-group.tsx";

/** Show a note search with a leading icon, its shortcut and a clear button. */
export function InputGroupExample(): JSX.Element {
    return (
        <InputGroup>
            <InputGroupInput type="search" placeholder="Search notes" aria-label="Search notes" />
            <InputGroupAddon>
                <InputGroupText>
                    <Icon name="magnifying-glass" />
                </InputGroupText>
            </InputGroupAddon>
            <InputGroupAddon align="inline-end">
                <Kbd>⌘K</Kbd>
                <InputGroupButton size="icon-xs" aria-label="Clear search">
                    <Icon name="x" />
                </InputGroupButton>
            </InputGroupAddon>
        </InputGroup>
    );
}
