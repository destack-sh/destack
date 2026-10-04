import { Icon } from "@destack/icon";
import type { JSX } from "@solidjs/web";
import { Button } from "./button.tsx";

/** Show a note's toolbar: a primary save, a quiet cancel and an icon-only delete. */
export function ButtonExample(): JSX.Element {
    return (
        <div role="toolbar" aria-label="Note">
            <Button type="submit">Save</Button>
            <Button variant="ghost">Cancel</Button>
            <Button variant="destructive" size="icon" aria-label="Delete note">
                <Icon name="trash" />
            </Button>
        </div>
    );
}
