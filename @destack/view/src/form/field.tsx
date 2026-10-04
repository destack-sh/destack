import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color } from "@destack/theme/tokens.stylex";
import { type Localization, t } from "@destack/locale";
import * as field from "@destack/ui/field";
import { Input } from "@destack/ui/input";
import { Select, SelectItem } from "@destack/ui/select";
import { Switch } from "@destack/ui/switch";
import { useLocale } from "@destack/locale/solid";
import type { JSX } from "../solid/component.ts";
import { For, Show } from "../solid/flow.ts";
import { createSignal, omit, type Accessor } from "../solid/reactive.ts";
import type { ObjectType } from "@destack/object";
import {
    columnOf,
    displayOf,
    isFieldValue,
    kindOf,
    messageOf,
    problemsOf,
    valueOf,
    type FieldBinding,
    type FieldName,
    type FieldStatus,
} from "./binding.ts";
import { StateTransition } from "./state-transition.tsx";

/** The properties of a field bound to an object field, a field's properties included. */
export interface FieldProperties<
    Object extends ObjectType,
    Name extends FieldName<Object>,
> extends Omit<field.FieldProperties, "value"> {
    /** The object field it shows and writes, choosing its control when it has no children. */
    readonly for: FieldBinding<Object, Name>;
    /** The label of the field. */
    readonly label?: string;
    /** The description under the field's control. */
    readonly description?: string;
}

/** Render a field bound to an object field: its control by type, its schema's refusals and its write's status. */
export function Field<Object extends ObjectType, Name extends FieldName<Object>>(
    properties: FieldProperties<Object, Name>,
): JSX.Element {
    // follow the last write and the last refusal
    const locale = useLocale();
    const rest = omit(properties, "for", "label", "description", "invalid", "children");
    const { problem, status, commit } = followWrites(() => properties.for, locale);
    const isLabelled = (): boolean =>
        "children" in properties || kindOf(properties.for) !== "state";

    return (
        <field.Field
            {...rest}
            invalid={
                properties.invalid === true || problem() !== undefined || status() === "failed"
            }
            value={{
                text: () => displayOf(kindOf(properties.for), properties.for.value),
                isChecked: () => properties.for.value === true,
                commit,
            }}
        >
            <Show when={properties.label}>
                {(label) => (
                    <Show
                        when={isLabelled()}
                        fallback={<field.FieldTitle>{label()}</field.FieldTitle>}
                    >
                        <field.FieldLabel>{label()}</field.FieldLabel>
                    </Show>
                )}
            </Show>
            <Show
                when={"children" in properties}
                fallback={<FieldControl binding={properties.for} />}
            >
                {properties.children}
            </Show>
            <Show when={properties.description}>
                {(description) => <field.FieldDescription>{description()}</field.FieldDescription>}
            </Show>
            <FieldStatusLine status={status()} />
            <field.FieldError>
                {problem() ??
                    (status() === "failed" ? locale.render(t`Could not save`) : undefined)}
            </field.FieldError>
        </field.Field>
    );
}

/** Validate committed values against a bound field's schema and write them, following the last write's answer. */
function followWrites<Object extends ObjectType, Name extends FieldName<Object>>(
    binding: () => FieldBinding<Object, Name>,
    locale: Localization,
): {
    problem: Accessor<string | undefined>;
    status: Accessor<FieldStatus>;
    commit: (raw: string | boolean) => void;
} {
    // follow the last refusal and the last write
    const [problem, setProblem] = createSignal<string | undefined>(undefined);
    const [status, setStatus] = createSignal<FieldStatus>("idle");
    let writes = 0;

    // validate and write each committed value
    const commit = (raw: string | boolean) => {
        // read the value the input stands for and the problems the field's schema reports
        const bound = binding();
        const value = valueOf(kindOf(bound), raw);
        const problems = problemsOf(bound, value);
        setProblem(problems.length === 0 ? undefined : messageOf(problems, locale));

        // write a kept value and follow the server's answer while no later write took over
        if (isFieldValue(bound, value)) {
            writes += 1;
            const write = writes;
            setStatus("pending");
            bound.write(value).confirmed.then(
                () => write === writes && setStatus("saved"),
                () => write === writes && setStatus("failed"),
            );
        }
    };

    return { problem, status, commit };
}

/** The styles of a field's status line. */
const styles = style.create({
    status: {
        margin: 0,
        minHeight: "1lh",
        color: color.mutedForeground,
    },
});

/** Render the line that announces a field's write status. */
function FieldStatusLine(properties: { readonly status: FieldStatus }): JSX.Element {
    const locale = useLocale();

    return (
        <p
            role="status"
            aria-live="polite"
            data-slot="field-status"
            {...style.attrs(text.footnote, styles.status)}
        >
            {properties.status === "pending"
                ? locale.render(t`Saving`)
                : properties.status === "saved"
                  ? locale.render(t`Saved`)
                  : ""}
        </p>
    );
}

/** Render the control an object field's declared type takes. */
function FieldControl(properties: { readonly binding: FieldBinding }): JSX.Element {
    const binding = properties.binding;
    const kind = kindOf(binding);
    if (kind === "boolean") {
        return <Switch />;
    } else if (kind === "choice") {
        return (
            <Select>
                <For each={columnOf(binding).enumValues ?? []}>
                    {(value) => <SelectItem value={value}>{value}</SelectItem>}
                </For>
            </Select>
        );
    } else if (kind === "state") {
        if (binding.access === undefined || binding.id === undefined) {
            throw new TypeError(
                `${binding.object.name}.${binding.field} needs the object's id and access for its transitions`,
            );
        }

        return (
            <StateTransition
                access={binding.access}
                object={binding.object}
                id={binding.id}
                field={binding.field}
                value={displayOf(kind, binding.value)}
            />
        );
    } else {
        return (
            <Input
                type={kind === "number" ? "number" : kind === "time" ? "datetime-local" : "text"}
            />
        );
    }
}
