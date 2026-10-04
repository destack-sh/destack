import { Icon } from "@destack/icon";
import type { JSX } from "@solidjs/web";
import { Tooltip, TooltipContent, TooltipTrigger } from "./tooltip.tsx";

/** Show what an icon-only archive button does. */
export function TooltipExample(): JSX.Element {
    return (
        <Tooltip>
            <TooltipTrigger variant="ghost" size="icon" aria-label="Archive">
                <Icon name="archive" />
            </TooltipTrigger>
            <TooltipContent>Archive note</TooltipContent>
        </Tooltip>
    );
}
