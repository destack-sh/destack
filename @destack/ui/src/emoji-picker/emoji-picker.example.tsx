import { defineExample } from "@destack/package/declare";
import { createSignal } from "@destack/view";
import {
    EmojiPicker,
    EmojiPickerContent,
    EmojiPickerFooter,
    EmojiPickerSearch,
    EmojiPickerSkinTone,
    EmojiPickerSkinToneSelector,
} from "./emoji-picker.tsx";

/** A reaction picked from every emoji by category, with a search, the skin tones and a note while nothing matches. */
export const emojiPickerReaction = defineExample({
    of: EmojiPicker,
    name: "reaction",
    description:
        "a reaction picked from every emoji by category, with a search, the skin tones and a note while nothing matches",
    render: () => (
        <EmojiPicker onEmojiSelect={() => undefined}>
            <EmojiPickerSearch />
            <EmojiPickerSkinTone />
            <EmojiPickerContent />
        </EmojiPicker>
    ),
});

/** A compact picker whose footer shows the active emoji beside a button cycling the skin tone. */
export const emojiPickerCompact = defineExample({
    of: EmojiPicker,
    name: "compact",
    description:
        "a compact picker whose footer shows the active emoji beside a button cycling the skin tone",
    render: () => (
        <EmojiPicker columns={7} onEmojiSelect={() => undefined}>
            <EmojiPickerSearch />
            <EmojiPickerContent />
            <EmojiPickerFooter>
                <EmojiPickerSkinToneSelector />
            </EmojiPickerFooter>
        </EmojiPicker>
    ),
});

/** A picker composed of its search, content and footer, reporting the emoji it picks. */
export const emojiPickerParts = defineExample({
    of: EmojiPicker,
    name: "parts",
    description:
        "a picker composed of its search, content and footer, reporting the emoji it picks",
    render: () => {
        const [picked, setPicked] = createSignal("");

        return (
            <>
                <EmojiPicker onEmojiSelect={(entry) => setPicked(entry.emoji)}>
                    <EmojiPickerSearch />
                    <EmojiPickerContent />
                    <EmojiPickerFooter />
                </EmojiPicker>
                <output>{picked()}</output>
            </>
        );
    },
});

/** A picker whose search finds no emoji, showing its note. */
export const emojiPickerEmpty = defineExample({
    of: EmojiPicker,
    name: "empty",
    description: "a picker whose search finds no emoji, showing its note",
    render: () => (
        <EmojiPicker defaultSearch="qqqqq" onEmojiSelect={() => undefined}>
            <EmojiPickerSearch />
            <EmojiPickerContent />
        </EmojiPicker>
    ),
});
