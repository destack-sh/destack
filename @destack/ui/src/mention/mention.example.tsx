import { t } from "@destack/locale";
import type { Choice } from "@destack/object";
import { defineExample } from "@destack/package/declare";
import { Mention } from "./mention.tsx";

/** A person, an at sign before their name. */
const ADA: Choice = { value: "ada", title: "Ada Lovelace" };

/** A page, its icon before its title, in its color. */
const LAUNCH: Choice = {
    value: "launch",
    title: "Launch plan",
    icon: { icon: "file-text" },
    color: "orange",
};

/** A person and a page mentioned in a sentence. */
export const mentionDefault = defineExample({
    of: Mention,
    name: "default",
    description: t`A person and a page mentioned in a sentence.`,
    layout: "padded",
    render: () => (
        <p>
            Ask <Mention choice={ADA} /> to read <Mention choice={LAUNCH} /> before Friday.
        </p>
    ),
});
