import { t } from "@destack/locale";
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

/** A reaction picked from every emoji by category. */
export const emojiPickerReaction = defineExample({
    of: EmojiPicker,
    name: "reaction",
    description: t`A reaction picked from every emoji by category, with a search, the skin tones and a note while nothing matches.`,
    properties: {
        columns: 9,
    },
    render: (properties) => (
        <EmojiPicker {...properties} onEmojiSelect={() => undefined}>
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
    description: t`A compact picker whose footer shows the active emoji beside a button cycling the skin tone.`,
    properties: {
        columns: 7,
    },
    render: (properties) => (
        <EmojiPicker {...properties} onEmojiSelect={() => undefined}>
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
    description: t`A picker composed of its search, content and footer, reporting the emoji it picks.`,
    render: (properties) => {
        const [picked, setPicked] = createSignal("");

        return (
            <>
                <EmojiPicker {...properties} onEmojiSelect={(entry) => setPicked(entry.emoji)}>
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
    description: t`A picker whose search finds no emoji, showing its note.`,
    render: (properties) => (
        <EmojiPicker {...properties} onEmojiSelect={() => undefined}>
            <EmojiPickerSearch defaultValue="qqqqq" />
            <EmojiPickerContent />
        </EmojiPicker>
    ),
});
