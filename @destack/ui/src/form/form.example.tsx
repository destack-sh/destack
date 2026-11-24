import { defineExample } from "@destack/package/declare";
import { schema } from "@destack/schema";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createSignal, Repeat, useEnvironment, useForm } from "@destack/view";
import { Button } from "../button/index.ts";
import { Checkbox } from "../checkbox/index.ts";
import { DateField } from "../date-field/index.ts";
import {
    Field,
    FieldConflict,
    FieldDescription,
    FieldError,
    FieldGroup,
    FieldLabel,
} from "../field/index.ts";
import { Input } from "../input/index.ts";
import { NumberField } from "../number-field/index.ts";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../select/index.ts";
import { Switch } from "../switch/index.ts";
import { Textarea } from "../textarea/index.ts";
import { Form, FormError, FormGuard, FormReset, FormStatus, FormSubmit } from "./form.tsx";

/** The styles of the examples. */
const styles = style.create({
    form: {
        width: "100%",
        maxWidth: "26rem",
    },
    row: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
});

/** What a trip needs. */
const Trip = schema.object({
    title: schema.string().min(1, "Give the trip a title").max(40),
    email: schema.email(),
    guests: schema.int().min(1).max(8),
    day: schema.string("Choose the day it starts"),
    room: schema.enum(["single", "double"]),
    note: schema.string().max(200),
    isInsured: schema.boolean(),
    isAgreed: schema.literal(true),
});

/** A trip as a person fills it in. */
interface TripDraft {
    /** What it is called. */
    readonly title: string;
    /** Where to write to. */
    readonly email: string;
    /** How many come. */
    readonly guests: number | undefined;
    /** The day it starts. */
    readonly day: string | undefined;
    /** The room. */
    readonly room: string | undefined;
    /** What else there is to say. */
    readonly note: string;
    /** Whether it is insured. */
    readonly isInsured: boolean;
    /** Whether the person agrees to the terms. */
    readonly isAgreed: boolean;
}

/** A trip checked by a schema, whose save fails the first time. */
export const formTrip = defineExample({
    of: Form,
    name: "trip",
    description: "a trip checked by a schema, whose save fails the first time",
    layout: "padded",
    render: () => {
        // save once, after a failure
        const [saved, setSaved] = createSignal<TripDraft>({
            title: "",
            email: "",
            guests: 2,
            day: undefined,
            room: undefined,
            note: "",
            isInsured: true,
            isAgreed: false,
        });
        const { clock } = useEnvironment();
        let tries = 0;
        const form = useForm<TripDraft>({
            values: saved,
            schema: Trip,
            submit: async (values) => {
                // take a moment, and fail once
                const { promise, resolve } = Promise.withResolvers<void>();
                clock.setTimeout(resolve, 600);
                await promise;
                tries += 1;
                if (tries === 1) {
                    throw new Error("offline");
                }
                setSaved(values);
            },
        });

        return (
            <Form form={form} xstyle={styles.form}>
                <FieldGroup>
                    <Field field={form.field("title")}>
                        <FieldLabel>Title</FieldLabel>
                        <Input />
                        <FieldError />
                    </Field>
                    <Field field={form.field("email")}>
                        <FieldLabel>Email</FieldLabel>
                        <Input type="email" autocomplete="email" />
                        <FieldDescription>We write to you once.</FieldDescription>
                        <FieldError />
                    </Field>
                    <Field field={form.field("guests")}>
                        <FieldLabel>Guests</FieldLabel>
                        <NumberField min={1} max={8} />
                        <FieldError />
                    </Field>
                    <Field field={form.field("day")}>
                        <FieldLabel>Day</FieldLabel>
                        <DateField />
                        <FieldError />
                    </Field>
                    <Field field={form.field("room")}>
                        <FieldLabel>Room</FieldLabel>
                        <Select placeholder="Choose a room">
                            <SelectTrigger>
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem value="single">Single</SelectItem>
                                <SelectItem value="double">Double</SelectItem>
                            </SelectContent>
                        </Select>
                        <FieldError />
                    </Field>
                    <Field field={form.field("note")}>
                        <FieldLabel>Note</FieldLabel>
                        <Textarea />
                        <FieldError />
                    </Field>
                    <Field field={form.field("isInsured")} orientation="horizontal">
                        <FieldLabel>Insurance</FieldLabel>
                        <Switch />
                    </Field>
                    <Field field={form.field("isAgreed")} orientation="horizontal">
                        <Checkbox />
                        <FieldLabel>I agree to the terms</FieldLabel>
                        <FieldError />
                    </Field>
                    <FormError />
                    <div {...style.attrs(styles.row)}>
                        <FormSubmit>Book</FormSubmit>
                        <FormReset>Start over</FormReset>
                        <FormStatus />
                    </div>
                </FieldGroup>
            </Form>
        );
    },
});

/** A list of guests, each a field of its own. */
export const formGuests = defineExample({
    of: Form,
    name: "guests",
    description: "a list of guests, each a field of its own",
    layout: "padded",
    render: () => {
        // keep the guests as they are submitted
        const [guests, setGuests] = createSignal<readonly { readonly name: string }[]>([
            { name: "Ada" },
            { name: "" },
        ]);
        const form = useForm<{ readonly guests: readonly { readonly name: string }[] }>({
            values: () => ({ guests: guests() }),
            schema: schema.object({
                guests: schema.array(schema.object({ name: schema.string().min(1) })).min(1),
            }),
            submit: (values) => {
                setGuests(values.guests);

                return undefined;
            },
        });
        const list = form.field("guests");

        return (
            <Form form={form} xstyle={styles.form}>
                <FieldGroup>
                    <Repeat count={list.value().length}>
                        {(index) => (
                            <Field field={list.item(index).field("name")}>
                                <FieldLabel>Guest {index + 1}</FieldLabel>
                                <div {...style.attrs(styles.row)}>
                                    <Input />
                                    <Button variant="ghost" onClick={() => list.remove(index)}>
                                        Remove
                                    </Button>
                                </div>
                                <FieldError />
                            </Field>
                        )}
                    </Repeat>
                    <div {...style.attrs(styles.row)}>
                        <FormSubmit>Save</FormSubmit>
                        <Button variant="outline" onClick={() => list.insert({ name: "" })}>
                            Add a guest
                        </Button>
                    </div>
                </FieldGroup>
            </Form>
        );
    },
});

/** A title someone else changes while a person edits it. */
export const formConflict = defineExample({
    of: FieldConflict,
    name: "conflict",
    description: "a title someone else changes while a person edits it",
    layout: "padded",
    render: () => {
        // change the title from outside
        const [title, setTitle] = createSignal("Groceries");
        const form = useForm({
            values: () => ({ title: title() }),
            submit: (values) => {
                setTitle(values.title);

                return undefined;
            },
        });

        return (
            <Form form={form} xstyle={styles.form}>
                <FieldGroup>
                    <Field field={form.field("title")}>
                        <FieldLabel>Title</FieldLabel>
                        <Input />
                        <FieldConflict />
                        <FieldError />
                    </Field>
                    <div {...style.attrs(styles.row)}>
                        <FormSubmit>Save</FormSubmit>
                        <Button variant="outline" onClick={() => setTitle("Market on Friday")}>
                            Change as someone else
                        </Button>
                    </div>
                </FieldGroup>
            </Form>
        );
    },
});

/** A form without a draft, whose fields show what the browser says. */
export const formNative = defineExample({
    of: Field,
    name: "native",
    description: "a form without a draft, whose fields show what the browser says",
    layout: "padded",
    render: () => (
        <form onSubmit={(event) => event.preventDefault()} {...style.attrs(styles.form)}>
            <FieldGroup>
                <Field>
                    <FieldLabel>Email</FieldLabel>
                    <Input type="email" name="email" required />
                    <FieldError />
                </Field>
                <Field>
                    <FieldLabel>Guests</FieldLabel>
                    <NumberField name="guests" required min={1} />
                    <FieldError />
                </Field>
                <Field>
                    <FieldLabel>Day</FieldLabel>
                    <DateField name="day" required />
                    <FieldError />
                </Field>
                <Button type="submit">Book</Button>
            </FieldGroup>
        </form>
    ),
});

/** A note whose edits outlive the page, and which asks before it is left. */
export const formDraft = defineExample({
    of: FormGuard,
    name: "draft",
    description: "a note whose edits outlive the page, and which asks before it is left",
    layout: "padded",
    render: () => {
        // hold a navigation as a router does
        const [isHeld, setHeld] = createSignal(false);
        const [note, setNote] = createSignal("");
        const form = useForm({
            values: () => ({ note: note() }),
            submit: (values) => {
                setNote(values.note);

                return undefined;
            },
            draft: { name: "example-note", schema: schema.object({ note: schema.string() }) },
        });

        return (
            <Form form={form} xstyle={styles.form}>
                <FieldGroup>
                    <Field field={form.field("note")}>
                        <FieldLabel>Note</FieldLabel>
                        <Textarea />
                    </Field>
                    <div {...style.attrs(styles.row)}>
                        <FormSubmit>Save</FormSubmit>
                        <FormReset>Discard</FormReset>
                        <Button variant="outline" onClick={() => setHeld(form.isEdited())}>
                            Leave
                        </Button>
                        <FormStatus />
                    </div>
                    <FormGuard
                        blocker={{
                            state: () => (isHeld() ? "blocked" : "unblocked"),
                            proceed: () => setHeld(false),
                            reset: () => setHeld(false),
                        }}
                    />
                </FieldGroup>
            </Form>
        );
    },
});
