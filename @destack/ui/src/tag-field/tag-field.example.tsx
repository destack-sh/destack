import { defineExample } from "@destack/package/declare";
import { createSignal } from "@destack/view";
import { Field, FieldDescription, FieldError, FieldLabel } from "../field/index.ts";
import { Stack } from "../stack/index.ts";
import {
    type TagRefusal,
    TagField,
    TagFieldClear,
    TagFieldInput,
    TagFieldList,
} from "./tag-field.tsx";

/** What a field says of a tag it does not take. */
const REFUSALS: Readonly<Record<TagRefusal, string>> = {
    duplicate: "is there already",
    max: "is one too many",
    invalid: "is not a mail address",
};

/** A note's tags. */
export const tagFieldNote = defineExample({
    of: TagField,
    name: "note",
    description: "a note's tags: type, paste, change in place and remove",
    layout: "padded",
    properties: { defaultValue: ["travel", "lisbon", "spring"] },
    render: (properties) => (
        <TagField {...properties}>
            <TagFieldList />
            <TagFieldInput aria-label="Add a tag" placeholder="Add a tag" />
            <TagFieldClear />
        </TagField>
    ),
});

/** The people of an invitation, whose addresses are checked. */
export const tagFieldInvite = defineExample({
    of: TagField,
    name: "invite",
    description: "the people of an invitation, whose addresses are checked, three at most",
    layout: "padded",
    render: () => {
        const [problem, setProblem] = createSignal("");

        return (
            <Field data-invalid={problem() === "" ? undefined : ""}>
                <FieldLabel>Invite</FieldLabel>
                <TagField
                    defaultValue={["ada@example.com"]}
                    max={3}
                    delimiter={/[,;\s]/u}
                    sanitizeValue={(tag) => tag.trim().toLowerCase()}
                    validate={(tag) => tag.includes("@")}
                    onValueInvalid={(tag, refusal) => setProblem(`${tag} ${REFUSALS[refusal]}`)}
                    onInputValueChange={() => setProblem("")}
                >
                    <TagFieldList />
                    <TagFieldInput inputmode="email" placeholder="name@example.com" />
                </TagField>
                <FieldDescription>Up to three people.</FieldDescription>
                <FieldError>{problem()}</FieldError>
            </Field>
        );
    },
});

/** Tags a person reads, and a field a person cannot use. */
export const tagFieldReadOnly = defineExample({
    of: TagField,
    name: "read-only",
    description: "tags a person reads, and a field a person cannot use",
    layout: "padded",
    render: () => (
        <Stack space="4">
            <TagField readOnly defaultValue={["travel", "lisbon"]} />
            <TagField disabled defaultValue={["travel", "lisbon"]} />
        </Stack>
    ),
});
