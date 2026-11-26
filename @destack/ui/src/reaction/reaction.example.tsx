import { t } from "@destack/locale";
import { EmojiButton } from "../example/emoji.tsx";
import { defineExample } from "@destack/package/declare";
import { createSignal, For } from "@destack/view";
import { Reaction, ReactionGroup } from "./reaction.tsx";

/** One's own name among the people who reacted. */
const OWN = "You";

/** The reactions to a message, by their emoji. */
const REACTIONS: Readonly<Record<string, readonly string[]>> = {
    "🎉": ["Ada", OWN, "Grace"],
    "👀": ["Katherine"],
    "🚀": ["Ada", "Grace", "Katherine", "Alan", "Barbara", "Edsger"],
};

/** Add one's own name to a list of people, or take it back. */
function toggled(people: readonly string[], isAdded: boolean): readonly string[] {
    return isAdded
        ? [...people.filter((name) => name !== OWN), OWN]
        : people.filter((name) => name !== OWN);
}

/** The reactions to a message, which a press adds one's own to. */
export const reactionGroupMessage = defineExample({
    of: ReactionGroup,
    name: "message",
    description: t`The reactions to a message, one's own among them, and a button for more.`,
    render: () => {
        const [reactions, setReactions] = createSignal(REACTIONS);
        const react = (emoji: string, isAdded: boolean): void => {
            setReactions((all) => ({ ...all, [emoji]: toggled(all[emoji] ?? [], isAdded) }));
        };

        return (
            <ReactionGroup>
                <For each={Object.keys(reactions())}>
                    {(emoji) => (
                        <Reaction
                            emoji={emoji}
                            count={reactions()[emoji]?.length ?? 0}
                            people={reactions()[emoji] ?? []}
                            pressed={reactions()[emoji]?.includes(OWN) === true}
                            onPressedChange={(isPressed) => react(emoji, isPressed)}
                        />
                    )}
                </For>
                <EmojiButton label="Add reaction" onSelect={(emoji) => react(emoji, true)} />
            </ReactionGroup>
        );
    },
});
