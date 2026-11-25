import check from "@destack/icon/phosphor/check";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { plural, t } from "@destack/locale";
import type { ActionInput, ActionValues, Choice } from "@destack/object";
import type { JsonValue } from "@destack/schema";
import {
    type Accessor,
    type ActionAsk,
    type ActionQuestion,
    createEffect,
    createMemo,
    createSignal,
    For,
    type JSX,
    onCleanup,
    Show,
    untrack,
    useAction,
    useActions,
    useContext,
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
import { Badge } from "../badge/index.ts";
import { Button } from "../button/index.ts";
import { ChoiceIcon } from "./choice-icon.tsx";
import {
    Command,
    CommandDialog,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandItemMeta,
    CommandList,
    CommandShortcut,
} from "../command/index.ts";
import {
    Dialog,
    DialogClose,
    DialogContent,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "../dialog/index.ts";
import { Icon } from "../icon/index.ts";
import { Field, FieldGroup, FieldLabel } from "../field/index.ts";
import { OpenContext } from "../layer/index.ts";
import { Popover, PopoverContent } from "../popover/index.ts";
import { ValueField } from "../value-field/index.ts";

/** The kinds of input a person picks from a list, however short. */
const PICKED_KINDS: ReadonlySet<string> = new Set(["object", "choice", "person"]);

/** The most choices digits pick from. */
const NUMBERED = 9;

/** The layout of the dialog's form. */
const styles = style.create({
    beside: {
        padding: 0,
    },
    subject: {
        display: "flex",
        paddingBlockStart: space[3],
        paddingInline: space[3],
    },
    subjectText: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    form: {
        display: "flex",
        flexDirection: "column",
        gap: space[4],
    },
    fields: {
        gap: space[4],
    },
});

/** A question on screen, the choices of its inputs, and its answer. */
interface Asking {
    /** The question. */
    readonly question: ActionQuestion;
    /** The choices of each input a person picks. */
    readonly choices: ReadonlyMap<string, readonly Choice[]>;
    /** What the action runs on, where something says so. */
    readonly subject: string | undefined;
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
    const ask: ActionAsk = async (question) => {
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

        // read what it runs on, and cancel a question on screen
        const subject = await actions.subjectTitle(action, element, locale);
        cancel();

        // answer once the person gives the values or cancels
        return new Promise<ActionValues | undefined>((resolve) => {
            const [isOpen, setOpen] = createSignal(true, { ownedWrite: true });
            setAsking({
                question,
                choices,
                subject,
                isOpen,
                answer: (values) => {
                    setOpen(false);
                    resolve(values);
                },
            });
        });
    };

    // ask while what is around it is open, as a closed dialog shows nothing
    createEffect(useContext(OpenContext), (isShown) =>
        isShown ? untrack(() => actions.asks(ask)) : undefined,
    );

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

/** Render an input's choices: beside the button that asked, else over the page. */
function ActionPick(properties: {
    /** The question. */
    readonly asking: Asking;
    /** The input to pick. */
    readonly input: ActionInput;
}): JSX.Element {
    // title the list by the action, and start on the value it has
    const locale = useLocale();
    const { question, answer } = properties.asking;
    const { action, element } = question;
    const title = () => locale.render(action.title);
    const own = useAction(() => ({ action, element, given: question.values }));
    const chosen = createMemo(() => own.chosen().map((value) => JSON.stringify(value)));
    const [first] = untrack(chosen);
    const start = first === undefined ? {} : { defaultValue: first };
    const close = (isOpen: boolean): void => {
        if (!isOpen) {
            answer(undefined);
        }
    };
    const list = () => (
        <ActionChoices asking={properties.asking} input={properties.input} chosen={chosen()} />
    );

    // stand beside the button that asked
    if (element instanceof HTMLButtonElement) {
        return (
            <Popover open={properties.asking.isOpen()} onOpenChange={close}>
                <PopoverContent anchor={element} align="start" xstyle={styles.beside}>
                    <Command aria-label={title()} {...start}>
                        {list()}
                    </Command>
                </PopoverContent>
            </Popover>
        );
    }

    return (
        <CommandDialog
            size="sm"
            open={properties.asking.isOpen()}
            title={title()}
            onOpenChange={close}
            {...start}
        >
            <Show when={properties.asking.subject}>
                {(subject) => (
                    <div data-slot="action-subject" {...style.attrs(styles.subject)}>
                        <Badge variant="secondary" xstyle={styles.subjectText}>
                            {subject()}
                        </Badge>
                    </div>
                )}
            </Show>
            {list()}
        </CommandDialog>
    );
}

/** Render the search and the list of an input's choices, those it has marked. */
function ActionChoices(properties: {
    /** The question. */
    readonly asking: Asking;
    /** The input to pick. */
    readonly input: ActionInput;
    /** The values it has, as JSON. */
    readonly chosen: readonly string[];
}): JSX.Element {
    // list the choices, and run the action with the one picked
    const locale = useLocale();
    const actions = useActions();
    const { question, answer } = properties.asking;
    const choices = () => properties.asking.choices.get(properties.input.name) ?? [];
    const pick = (choice: Choice) => {
        // run it for this choice and stay open, or answer with it
        const values = { [properties.input.name]: choice.value };
        if (question.action.isRepeated) {
            actions.start(question.action, question.element, { ...question.values, ...values });
        } else {
            answer(values);
        }
    };

    // number a short list, unless its titles have digits a person searches for
    const isNumbered = () =>
        choices().length <= NUMBERED && choices().every((choice) => !/\d/u.test(choice.title));
    const digit = (event: KeyboardEvent) => {
        // pick the choice of a digit typed into an empty search
        const isPlain = !event.metaKey && !event.ctrlKey && !event.altKey;
        const choice = /^[1-9]$/u.test(event.key) ? choices()[Number(event.key) - 1] : undefined;
        const { target } = event;
        const isEmpty = target instanceof HTMLInputElement && target.value === "";
        if (isNumbered() && isPlain && isEmpty && choice !== undefined) {
            event.preventDefault();
            pick(choice);
        }
    };

    return (
        <>
            <CommandInput placeholder={locale.render(question.action.title)} onKeyDown={digit} />
            <CommandList>
                <CommandEmpty>{locale.render(t`No results.`)}</CommandEmpty>
                <CommandGroup>
                    <For each={choices()}>
                        {(choice, index) => (
                            <CommandItem
                                value={JSON.stringify(choice.value)}
                                keywords={[choice.title]}
                                icon={<ChoiceIcon choice={choice} />}
                                onSelect={() => pick(choice)}
                            >
                                {choice.title}
                                <CommandItemMeta>
                                    <Show
                                        when={properties.chosen.includes(
                                            JSON.stringify(choice.value),
                                        )}
                                    >
                                        <Icon icon={check} label={locale.render(t`Chosen`)} />
                                    </Show>
                                    <Show when={isNumbered()}>
                                        <CommandShortcut keys={String(index() + 1)} />
                                    </Show>
                                </CommandItemMeta>
                            </CommandItem>
                        )}
                    </For>
                </CommandGroup>
            </CommandList>
        </>
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
                                    value={values()[input.name]}
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
    /** The value given, undefined for none. */
    readonly value: JsonValue | undefined;
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
            <ValueField
                type={value}
                value={properties.value}
                onValueChange={onGive}
                choices={choices}
                required={presence === "required"}
            />
        </Field>
    );
}
