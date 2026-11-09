import { argument, type Message, t } from "@destack/locale";
import { createSignal, For, type JSX, Show, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import { Field, FieldDescription, FieldError, FieldLabel } from "../field/index.ts";
import { Input } from "../input/index.ts";
import { Stack } from "../stack/index.ts";

/** Whether a handle is free, taken by an account or a lookalike, or not a handle. */
export type HandleAvailability = "available" | "taken" | "invalid";

/** What a person reads about a handle that is not free. */
const HANDLE_REFUSALS: Readonly<Record<Exclude<HandleAvailability, "available">, Message>> = {
    taken: t`This handle is taken.`,
    invalid: t`Use lowercase letters and digits, with single hyphens between them.`,
};

/** A residency a person stores their data in, by its code and the name they read. */
export interface SignUpResidencyOption {
    /** The residency's code, such as eu. */
    readonly code: string;
    /** The name people read, such as European Union. */
    readonly name: string;
}

/** The properties of the handle field. */
export interface SignUpHandleProperties {
    /** The handle as typed. */
    readonly value: string;
    /** Whether the typed handle is free, absent while unchecked. */
    readonly availability?: HandleAvailability | undefined;
    /** Check a typed handle. */
    readonly onInput: (handle: string) => void;
}

/** Render the handle field with the profile address it gives and why a handle is not free. */
export function SignUpHandle(properties: SignUpHandleProperties): JSX.Element {
    const locale = useLocale();
    const refusal = () =>
        properties.availability === undefined || properties.availability === "available"
            ? undefined
            : locale.render(HANDLE_REFUSALS[properties.availability]);

    return (
        <Field invalid={refusal() !== undefined}>
            <FieldLabel>{locale.render(t`Handle`)}</FieldLabel>
            <Input
                name="handle"
                autocapitalize="none"
                spellcheck={false}
                required
                value={properties.value}
                onInput={(event) =>
                    properties.onInput(event.currentTarget.value.trim().toLowerCase())
                }
            />
            <FieldDescription>
                {locale.render(t`destack.app/@${argument("handle")}`, {
                    handle: properties.value === "" ? locale.render(t`handle`) : properties.value,
                })}
            </FieldDescription>
            <FieldError errors={[{ message: refusal() }]} />
        </Field>
    );
}

/** The properties of the residency line. */
export interface SignUpResidencyProperties {
    /** The residencies to choose from. */
    readonly residencies: readonly SignUpResidencyOption[];
    /** The chosen residency's code, absent before a choice. */
    readonly value?: string | undefined;
    /** Choose a residency. */
    readonly onChange: (code: string) => void;
}

/** Render where the person's data lives as one line, its choices open only to change it. */
export function SignUpResidency(properties: SignUpResidencyProperties): JSX.Element {
    // show the choices until one is made
    const locale = useLocale();
    const [isChoosing, setChoosing] = createSignal(properties.value === undefined);
    const chosen = () => properties.residencies.find((each) => each.code === properties.value);

    return (
        <Show
            when={!isChoosing() && chosen()}
            fallback={
                <fieldset>
                    <legend>{locale.render(t`Where your data lives`)}</legend>
                    <Stack space="2">
                        <For each={properties.residencies}>
                            {(residency) => (
                                <Button
                                    variant={
                                        residency.code === properties.value ? "default" : "outline"
                                    }
                                    onClick={() => {
                                        // choose it and fold the choices back into the line
                                        properties.onChange(residency.code);
                                        setChoosing(false);
                                    }}
                                >
                                    {residency.name}
                                </Button>
                            )}
                        </For>
                    </Stack>
                </fieldset>
            }
        >
            {(residency) => (
                <p>
                    {locale.render(t`Your data lives in the ${argument("residency")}`, {
                        residency: residency().name,
                    })}{" "}
                    ·{" "}
                    <Button variant="link" onClick={() => setChoosing(true)}>
                        {locale.render(t`change`)}
                    </Button>
                </p>
            )}
        </Show>
    );
}
