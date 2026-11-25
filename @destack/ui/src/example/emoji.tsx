import smileyIcon from "@destack/icon/phosphor/smiley";
import { createSignal, type JSX, merge } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { EmojiPicker, EmojiPickerContent, EmojiPickerSearch } from "../emoji-picker/index.ts";
import { Icon } from "../icon/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";

/** Render the examples' button that picks an emoji in a popover. */
export function EmojiButton(properties: {
    /** What it does, such as adding a reaction. */
    readonly label: string;
    /** Handle the emoji picked. */
    readonly onSelect: (emoji: string) => void;
    /** Render the button itself, such as in a toolbar. */
    readonly render?: (properties: ButtonProperties) => JSX.Element;
}): JSX.Element {
    // close the popover with the emoji picked
    const [isOpen, setOpen] = createSignal(false);

    return (
        <Popover open={isOpen()} onOpenChange={(next) => void setOpen(next)}>
            <PopoverTrigger
                render={(attributes) => {
                    // fill the button the caller renders, or a round one
                    const button: ButtonProperties = merge(attributes, {
                        variant: "ghost" as const,
                        size: "icon-sm" as const,
                        get "aria-label"() {
                            return properties.label;
                        },
                        get children() {
                            return <Icon icon={smileyIcon} />;
                        },
                    });

                    return properties.render?.(button) ?? <Button {...button} shape="round" />;
                }}
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
