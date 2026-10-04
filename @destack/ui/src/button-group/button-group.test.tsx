import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import { Button } from "../button/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { ButtonGroup, ButtonGroupSeparator } from "./index.ts";

test("render a group of buttons with a vertical separator between them", () => {
    const container = draw(() => (
        <ButtonGroup aria-label="Save">
            <Button>Save</Button>
            <ButtonGroupSeparator />
        </ButtonGroup>
    ));
    expect(markup(container)).toBe(
        '<div data-slot="button-group" data-orientation="horizontal" role="group" aria-label="Save">' +
            '<button data-slot="button" data-variant="default" data-size="default">Save</button>' +
            '<div data-slot="button-group-separator" data-orientation="vertical" role="none"></div></div>',
    );
});

test("join a group's buttons and leave the buttons inside its popover apart", () => {
    const container = draw(() => (
        <>
            <Button>Alone</Button>
            <ButtonGroup>
                <Button>Joined</Button>
                <Popover>
                    <PopoverTrigger>More</PopoverTrigger>
                    <PopoverContent>
                        <Button>Inside</Button>
                    </PopoverContent>
                </Popover>
            </ButtonGroup>
        </>
    ));
    const [alone, joined, inside] = ["Alone", "Joined", "Inside"].map(
        (text) =>
            [...container.querySelectorAll("button")].find((button) => button.textContent === text)
                ?.className,
    );
    expect([joined === alone, inside === alone]).toEqual([false, true]);
});
