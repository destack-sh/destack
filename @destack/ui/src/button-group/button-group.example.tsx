import { Icon } from "@destack/icon";
import type { JSX } from "@solidjs/web";
import { Button } from "../button/index.ts";
import { ButtonGroup, ButtonGroupSeparator } from "./button-group.tsx";

/** Show a note's split save button: save, and a menu button for the other ways to save. */
export function ButtonGroupExample(): JSX.Element {
    return (
        <ButtonGroup aria-label="Save">
            <Button>Save</Button>
            <ButtonGroupSeparator />
            <Button size="icon" aria-label="More ways to save">
                <Icon name="caret-down" />
            </Button>
        </ButtonGroup>
    );
}
