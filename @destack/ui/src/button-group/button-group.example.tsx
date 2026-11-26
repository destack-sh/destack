import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import { Button } from "../button/index.ts";
import { ButtonGroup, ButtonGroupSeparator } from "./button-group.tsx";

/** A note's split save button: save, and a menu button for the other ways to save. */
export const buttonGroupSplitSave = defineExample({
    of: ButtonGroup,
    name: "splitSave",
    description: t`A note's split save button: save, and a menu button for the other ways to save.`,
    properties: {
        "aria-label": "Save",
    },
    render: (properties) => (
        <ButtonGroup {...properties}>
            <Button>Save</Button>
            <ButtonGroupSeparator />
            <Button isIconOnly aria-label="More ways to save">
                <Icon name="caret-down" />
            </Button>
        </ButtonGroup>
    ),
});

/** The split save button stacked. */
export const buttonGroupSplitSaveVertical = defineExample({
    of: ButtonGroup,
    name: "splitSaveVertical",
    description: t`The split save button stacked.`,
    properties: {
        orientation: "vertical",
        "aria-label": "Save",
    },
    render: (properties) => (
        <ButtonGroup {...properties}>
            <Button>Save</Button>
            <ButtonGroupSeparator />
            <Button isIconOnly aria-label="More ways to save">
                <Icon name="caret-down" />
            </Button>
        </ButtonGroup>
    ),
});
