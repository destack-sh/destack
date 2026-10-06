import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/icon";
import { Button } from "../button/index.ts";
import { ButtonGroup, ButtonGroupSeparator } from "./button-group.tsx";

/** A note's split save button: save, and a menu button for the other ways to save. */
export const buttonGroupSplitSave = defineExample({
    of: ButtonGroup,
    name: "split-save",
    description: "a note's split save button: save, and a menu button for the other ways to save",
    render: () => (
        <ButtonGroup aria-label="Save">
            <Button>Save</Button>
            <ButtonGroupSeparator />
            <Button size="icon" aria-label="More ways to save">
                <Icon name="caret-down" />
            </Button>
        </ButtonGroup>
    ),
});
