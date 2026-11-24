import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { plural, t } from "@destack/locale";
import type { ActionInput, ActionValue, ActionValues, Choice } from "@destack/object";
import { type JsonValue, type PlainDate, type PlainTime, TimeZone } from "@destack/schema";
import {
    type Accessor,
    type ActionQuestion,
    createSignal,
    For,
    handleAsking,
    type JSX,
    onCleanup,
    Show,
    untrack,
    useActions,
    useEnvironment,
    useLocale,
} from "@destack/view";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from "../alert-dialog/index.ts";
import { Button } from "../button/index.ts";
import { ChoiceIcon } from "./choice-icon.tsx";
import {
    CommandDialog,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
} from "../command/index.ts";
import {
    Dialog,
    DialogClose,
    DialogContent,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "../dialog/index.ts";
import { Field, FieldGroup, FieldLabel } from "../field/index.ts";
import { Input } from "../input/index.ts";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../select/index.ts";
import { Switch } from "../switch/index.ts";
import { DateField } from "../date-field/index.ts";
import { TimeField } from "../time-field/index.ts";

/** The kinds of input a person picks from a list, however short. */
const PICKED_KINDS: ReadonlySet<string> = new Set(["object", "choice", "person"]);

/** The layout of the dialog's form. */
const styles = style.create({
    form: {
        display: "flex",
        flexDirection: "column",
        gap: space[4],
    },
    fields: {
        gap: space[4],
    },
    control: {
        width: "100%",
    },
    half: {
        flex: "1 1 0",
    },
    time: {
        display: "flex",
        flexWrap: "wrap",
        gap: space[2],
    },
});

/** A question on screen, the choices of its inputs, and its answer. */
interface Asking {
    /** The question. */
    readonly question: ActionQuestion;
    /** The choices of each input a person picks. */
    readonly choices: ReadonlyMap<string, readonly Choice[]>;
    /** Whether the question is on screen, closing once answered. */
    readonly isOpen: Accessor<boolean>;
    /** Answer the question, cancelled without values. */
    readonly answer: (values: ActionValues | undefined) => void;
}

/** Ask for what actions still need: a list for one choice, a form, and to confirm removals. */
export function ActionDialog(): JSX.Element {
    // show one question at a time, with the choices of its inputs
    const actions = useActions();
    const locale = useLocale();
    const [asking, setAsking] = createSignal<Asking | undefined>(undefined, { ownedWrite: true });
    const cancel = () => untrack(asking)?.answer(undefined);
    onCleanup(cancel);
    handleAsking(async (question) => {
        // read the choices of each input
        const { action, asked, element } = question;
        const choices = new Map(
            await Promise.all(
                asked.map(
                    async (input) =>
                        [
                            input.name,
                            await actions.choices(action, input, element, locale),
                        ] as const,
                ),
            ),
        );

        // cancel a question on screen, and answer once the person gives the values or cancels
        cancel();

        return new Promise<ActionValues | undefined>((resolve) => {
            const [isOpen, setOpen] = createSignal(true, { ownedWrite: true });
            setAsking({
                question,
                choices,
                isOpen,
                answer: (values) => {
                    setOpen(false);
                    resolve(values);
                },
            });
        });
    });

    // pick the one input left where it has choices
    const picked = (current: Asking): ActionInput | undefined => {
        const [single, ...others] = current.question.asked;
        const isListed =
            single !== undefined &&
            (PICKED_KINDS.has(single.value.kind) ||
                (current.choices.get(single.name) ?? []).length > 0);

        return others.length === 0 && isListed ? single : undefined;
    };

    return (
        <Show when={asking()} keyed>
            {(current) => (
                <Show
                    when={current.question.asked.length > 0}
                    fallback={<ActionConfirm asking={current} />}
                >
                    <Show when={picked(current)} fallback={<ActionForm asking={current} />}>
                        {(input) => <ActionPick asking={current} input={input()} />}
                    </Show>
                </Show>
            )}
        </Show>
    );
}

/** Render a searchable list of an input's choices. */
function ActionPick(properties: {
    /** The question. */
    readonly asking: Asking;
    /** The input to pick. */
    readonly input: ActionInput;
}): JSX.Element {
    // title the list by the action
    const locale = useLocale();
    const { question, answer } = properties.asking;
    const title = () => locale.render(question.action.title);

    return (
        <CommandDialog
            size="sm"
            open={properties.asking.isOpen()}
            title={title()}
            onOpenChange={(isOpen) => {
                if (!isOpen) {
                    answer(undefined);
                }
            }}
        >
            <CommandInput placeholder={title()} />
            <CommandList>
                <CommandEmpty>{locale.render(t`No results.`)}</CommandEmpty>
                <CommandGroup>
                    <For each={properties.asking.choices.get(properties.input.name) ?? []}>
                        {(choice) => (
                            <CommandItem
                                value={JSON.stringify(choice.value)}
                                keywords={[choice.title]}
                                icon={<ChoiceIcon choice={choice} />}
                                onSelect={() => answer({ [properties.input.name]: choice.value })}
                            >
                                {choice.title}
                            </CommandItem>
                        )}
                    </For>
                </CommandGroup>
            </CommandList>
        </CommandDialog>
    );
}

/** Render a confirmation before an action removes for good. */
function ActionConfirm(properties: {
    /** The question. */
    readonly asking: Asking;
}): JSX.Element {
    // count the objects it removes
    const locale = useLocale();
    const { question, answer } = properties.asking;
    const { action, values, element } = question;
    const actions = useActions();
    const target = action.call?.target;
    const given = target === undefined ? undefined : values[target];
    const count = Array.isArray(given) ? given.length : 1;
    const title = locale.render(action.title);
    const sentence = actions.sentence(action, element, locale, values);

    return (
        <AlertDialog
            open={properties.asking.isOpen()}
            onOpenChange={(isOpen) => {
                if (!isOpen) {
                    answer(undefined);
                }
            }}
        >
            <AlertDialogContent>
                <AlertDialogHeader>
                    <AlertDialogTitle>{locale.render(t`${sentence}?`)}</AlertDialogTitle>
                    <AlertDialogDescription>
                        {locale.render(
                            t`${plural(count, { one: "This cannot be undone.", other: "This affects # items and cannot be undone." })}`,
                        )}
                    </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogCancel>{locale.render(t`Cancel`)}</AlertDialogCancel>
                    <AlertDialogAction variant="destructive" onClick={() => answer({})}>
                        {title}
                    </AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    );
}

/** Render a form of an action's inputs. */
function ActionForm(properties: {
    /** The question. */
    readonly asking: Asking;
}): JSX.Element {
    // collect a value for each input, answering with those given
    const locale = useLocale();
    const { question, choices, answer } = properties.asking;
    const { action, asked } = question;
    const [values, setValues] = createSignal<ActionValues>(
        Object.fromEntries(
            asked.flatMap(({ name, presence, value }) =>
                presence === "required" && value.kind === "boolean" ? [[name, false]] : [],
            ),
        ),
    );
    const give = (name: string, value: JsonValue | undefined) =>
        setValues((current) => {
            const { [name]: _dropped, ...rest } = current;

            return value === undefined ? rest : { ...rest, [name]: value };
        });
    const isComplete = () =>
        asked.every(
            ({ name, presence }) => presence !== "required" || values()[name] !== undefined,
        );

    return (
        <Dialog
            open={properties.asking.isOpen()}
            onOpenChange={(isOpen) => {
                if (!isOpen) {
                    answer(undefined);
                }
            }}
        >
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>{locale.render(action.title)}</DialogTitle>
                </DialogHeader>
                <form
                    {...style.attrs(styles.form)}
                    onSubmit={(event) => {
                        event.preventDefault();
                        answer(values());
                    }}
                >
                    <FieldGroup xstyle={styles.fields}>
                        <For each={asked}>
                            {(input) => (
                                <ActionField
                                    input={input}
                                    choices={choices.get(input.name) ?? []}
                                    onGive={(value) => give(input.name, value)}
                                />
                            )}
                        </For>
                    </FieldGroup>
                    <DialogFooter>
                        <DialogClose
                            render={(attributes) => <Button {...attributes} variant="outline" />}
                        >
                            {locale.render(t`Cancel`)}
                        </DialogClose>
                        <Button type="submit" disabled={!isComplete()}>
                            {locale.render(action.title)}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}

/** Render the field of one input, by what it takes. */
function ActionField(properties: {
    /** The input. */
    readonly input: ActionInput;
    /** The choices a person picks it from, none for one they type. */
    readonly choices: readonly Choice[];
    /** Take the value given, undefined for none. */
    readonly onGive: (value: JsonValue | undefined) => void;
}): JSX.Element {
    // label it by its own label, else by its type's title or its name
    const locale = useLocale();
    const { input, choices, onGive } = properties;
    const { value, presence } = input;
    const label =
        input.label !== undefined
            ? locale.render(input.label)
            : value.kind === "object"
              ? locale.render(value.type.declared.title, { count: 1 })
              : input.name;

    return (
        <Field orientation={value.kind === "boolean" ? "horizontal" : "vertical"}>
            <FieldLabel>{label}</FieldLabel>
            <ActionControl
                value={value}
                isRequired={presence === "required"}
                choices={choices}
                onGive={onGive}
            />
        </Field>
    );
}

/** Render the control of a value, by its kind. */
function ActionControl(properties: {
    /** The kind of value. */
    readonly value: ActionValue;
    /** Whether the person must give it. */
    readonly isRequired: boolean;
    /** The choices a person picks it from, none for one they type. */
    readonly choices: readonly Choice[];
    /** Take the value given, undefined for none. */
    readonly onGive: (value: JsonValue | undefined) => void;
}): JSX.Element {
    // switch a yes or no, and take a time as its day and time of day
    const locale = useLocale();
    const { value, isRequired, choices, onGive } = properties;
    const isListed = choices.length > 0 || PICKED_KINDS.has(value.kind);
    if (value.kind === "boolean") {
        return <Switch onCheckedChange={(isOn) => onGive(isOn)} />;
    } else if (value.kind === "time") {
        return <ActionTime onGive={onGive} />;
    }
    // choose one of the choices
    else if (isListed) {
        return (
            <Select
                xstyle={styles.control}
                required={isRequired}
                onValueChange={(chosen) =>
                    onGive(choices.find((choice) => JSON.stringify(choice.value) === chosen)?.value)
                }
            >
                <SelectTrigger>
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    <SelectItem value="">{locale.render(t`Choose…`)}</SelectItem>
                    <For each={choices}>
                        {(choice) => (
                            <SelectItem value={JSON.stringify(choice.value)}>
                                {choice.title}
                            </SelectItem>
                        )}
                    </For>
                </SelectContent>
            </Select>
        );
    }

    // type a number or text
    const isNumber = value.kind === "number";

    return (
        <Input
            required={isRequired}
            type={isNumber ? "number" : "text"}
            onInput={({ currentTarget: field }) => {
                const typed = isNumber ? field.valueAsNumber : field.value;
                onGive(field.value === "" ? undefined : typed);
            }}
        />
    );
}

/** Render the fields of a time: its day and time of day, in the reader's zone. */
function ActionTime(properties: {
    /** Take the moment given, undefined for none. */
    readonly onGive: (value: JsonValue | undefined) => void;
}): JSX.Element {
    // give the moment once both are set, else none
    const { timeZone } = useEnvironment();
    let day: PlainDate | undefined;
    let time: PlainTime | undefined;
    const give = () =>
        properties.onGive(
            day === undefined || time === undefined
                ? undefined
                : TimeZone.instant(timeZone, day, time),
        );

    return (
        <div {...style.attrs(styles.time)}>
            <DateField
                xstyle={styles.half}
                onValueChange={(given) => {
                    day = given;
                    give();
                }}
            />
            <TimeField
                xstyle={styles.half}
                onValueChange={(given) => {
                    time = given;
                    give();
                }}
            />
        </div>
    );
}
