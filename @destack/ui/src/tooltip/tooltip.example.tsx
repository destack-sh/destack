import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/icon";
import { Tooltip, TooltipArrow, TooltipContent, TooltipTrigger } from "./tooltip.tsx";

/** What an icon-only archive button does. */
export const tooltipArchiveButton = defineExample({
    of: Tooltip,
    name: "archive-button",
    description: "what an icon-only archive button does",
    render: () => (
        <Tooltip>
            <TooltipTrigger variant="ghost" size="icon" aria-label="Archive">
                <Icon name="archive" />
            </TooltipTrigger>
            <TooltipContent>Archive note</TooltipContent>
        </Tooltip>
    ),
});

/** The archive button's tooltip shown above it. */
export const tooltipArchiveButtonOpen = defineExample({
    of: Tooltip,
    name: "archive-button-open",
    description: "the archive button's tooltip shown above it",
    render: () => (
        <Tooltip defaultOpen>
            <TooltipTrigger variant="ghost" size="icon" aria-label="Archive">
                <Icon name="archive" />
            </TooltipTrigger>
            <TooltipContent>Archive note</TooltipContent>
        </Tooltip>
    ),
});

/** The archive button's tooltip shown above it with an arrow pointing at the button. */
export const tooltipArchiveButtonArrow = defineExample({
    of: Tooltip,
    name: "archive-button-arrow",
    description: "the archive button's tooltip shown above it with an arrow pointing at the button",
    render: () => (
        <Tooltip defaultOpen>
            <TooltipTrigger variant="ghost" size="icon" aria-label="Archive">
                <Icon name="archive" />
            </TooltipTrigger>
            <TooltipContent>
                <TooltipArrow />
                Archive note
            </TooltipContent>
        </Tooltip>
    ),
});
