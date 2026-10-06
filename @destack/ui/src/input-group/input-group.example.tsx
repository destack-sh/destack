import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/icon";
import { Kbd } from "../kbd/index.ts";
import {
    InputGroup,
    InputGroupAddon,
    InputGroupButton,
    InputGroupInput,
    InputGroupText,
} from "./input-group.tsx";

/** A note search with a leading icon, its shortcut and a clear button. */
export const inputGroupNoteSearch = defineExample({
    of: InputGroup,
    name: "note-search",
    description: "a note search with a leading icon, its shortcut and a clear button",
    render: () => (
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
    ),
});
