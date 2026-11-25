import smileyIcon from "@destack/icon/phosphor/smiley";
import { createSignal, type JSX } from "@destack/view";
import { Button } from "../button/index.ts";
import { EmojiPicker, EmojiPickerContent, EmojiPickerSearch } from "../emoji-picker/index.ts";
import { Icon } from "../icon/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";

/** Render the examples' button that picks an emoji in a popover. */
export function EmojiButton(properties: {
    /** What it does, such as adding a reaction. */
    readonly label: string;
    /** Handle the emoji picked. */
    readonly onSelect: (emoji: string) => void;
}): JSX.Element {
    // close the popover with the emoji picked
    const [isOpen, setOpen] = createSignal(false);

    return (
        <Popover open={isOpen()} onOpenChange={(next) => void setOpen(next)}>
            <PopoverTrigger
                render={(attributes) => (
                    <Button
                        {...attributes}
                        variant="ghost"
                        size="icon-sm"
                        shape="round"
                        aria-label={properties.label}
                    >
                        <Icon icon={smileyIcon} />
                    </Button>
                )}
            />
            <PopoverContent>
                <EmojiPicker
                    onEmojiSelect={({ emoji }) => {
                        setOpen(false);
                        properties.onSelect(emoji);
                    }}
                >
                    <EmojiPickerSearch />
                    <EmojiPickerContent />
                </EmojiPicker>
            </PopoverContent>
        </Popover>
    );
}
