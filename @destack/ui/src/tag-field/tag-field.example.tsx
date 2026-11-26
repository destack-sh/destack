import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { createSignal } from "@destack/view";
import { Field, FieldDescription, FieldError, FieldLabel } from "../field/index.ts";
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
    description: t`A note's tags: type, paste, change in place and remove.`,
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
    description: t`The people of an invitation, whose addresses are checked, three at most.`,
    layout: "padded",
    render: (properties) => {
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
                    {...properties}
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
