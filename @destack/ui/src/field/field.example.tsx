import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createSignal } from "@destack/view";
import { Button } from "../button/index.ts";
import { Checkbox } from "../checkbox/index.ts";
import { Input } from "../input/index.ts";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../select/index.ts";
import { Slider } from "../slider/index.ts";
import { Switch } from "../switch/index.ts";
import { Textarea } from "../textarea/index.ts";
import {
    Field,
    FieldContent,
    FieldDescription,
    FieldError,
    FieldGroup,
    FieldLabel,
    FieldLegend,
    FieldSet,
} from "./field.tsx";

/** The width of a form. */
const styles = style.create({
    form: {
        maxWidth: "28rem",
        marginInline: "auto",
    },
    wide: {
        width: "100%",
    },
    actions: {
        display: "flex",
        justifyContent: "flex-end",
        gap: space[2],
    },
});

/** A notebook form whose name field reports an error until it has a value. */
export const fieldNotebookForm = defineExample({
    of: Field,
    layout: "padded",
    name: "notebookForm",
    description: t`A notebook form whose name field reports an error until it has a value.`,
    render: (properties) => {
        const [name, setName] = createSignal("");
        const error = (): string | undefined => (name() === "" ? "Enter a name" : undefined);

        return (
            <FieldSet xstyle={styles.form}>
                <FieldLegend>Notebook</FieldLegend>
                <FieldGroup>
                    <Field invalid={error() !== undefined}>
                        <FieldLabel>Name</FieldLabel>
                        <Input
                            value={name()}
                            onInput={(event) => setName(event.currentTarget.value)}
                        />
                        <FieldError errors={[{ message: error() }]} />
                    </Field>
                    <Field {...properties}>
                        <FieldLabel>Description</FieldLabel>
                        <Textarea />
                        <FieldDescription>
                            Shown to everyone you share the notebook with.
                        </FieldDescription>
                    </Field>
                </FieldGroup>
            </FieldSet>
        );
    },
});

/** An event's settings: every control with its label, its help and its error. */
export const fieldEventForm = defineExample({
    of: Field,
    name: "eventForm",
    description: t`An event's settings: every control with its label, its help and its error.`,
    layout: "padded",
    render: (properties) => {
        // refuse an event without seats
        const [seats, setSeats] = createSignal("0");
        const isFull = (): boolean => Number(seats()) < 1;

        return (
            <FieldSet xstyle={styles.form}>
                <FieldLegend>Launch party</FieldLegend>
                <FieldDescription>Guests see these once you send the invitations.</FieldDescription>
                <FieldGroup>
                    {/* Text */}
                    <Field {...properties}>
                        <FieldLabel>Title</FieldLabel>
                        <Input value="Launch party" />
                    </Field>
                    <Field>
                        <FieldLabel>Description</FieldLabel>
                        <Textarea value="Drinks from seven, the demo at eight." />
                        <FieldDescription>Shown on the invitation.</FieldDescription>
                    </Field>

                    {/* Choice */}
                    <Field>
                        <FieldLabel>Room</FieldLabel>
                        <Select xstyle={styles.wide}>
                            <SelectTrigger>
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem value="atrium">Atrium</SelectItem>
                                <SelectItem value="roof">Roof terrace</SelectItem>
                            </SelectContent>
                        </Select>
                    </Field>
                    <Field invalid={isFull()}>
                        <FieldLabel>Seats</FieldLabel>
                        <Input
                            type="number"
                            min="0"
                            value={seats()}
                            onInput={(event) => setSeats(event.currentTarget.value)}
                        />
                        <FieldError
                            errors={[{ message: isFull() ? "Enter at least one seat" : undefined }]}
                        />
                    </Field>
                    <Field>
                        <FieldLabel>Volume</FieldLabel>
                        <Slider defaultValue={[40]} marks={[0, 25, 50, 75, 100]} />
                    </Field>

                    {/* Yes or no */}
                    <Field orientation="horizontal">
                        <Checkbox defaultChecked />
                        <FieldContent>
                            <FieldLabel>Guests may bring someone</FieldLabel>
                            <FieldDescription>Each guest adds one name.</FieldDescription>
                        </FieldContent>
                    </Field>
                    <Field orientation="horizontal">
                        <FieldContent>
                            <FieldLabel>Remind guests</FieldLabel>
                            <FieldDescription>A day before the event.</FieldDescription>
                        </FieldContent>
                        <Switch defaultChecked />
                    </Field>
                </FieldGroup>

                {/* Actions */}
                <div {...style.attrs(styles.actions)}>
                    <Button variant="outline">Cancel</Button>
                    <Button>Save</Button>
                </div>
            </FieldSet>
        );
    },
});

/** The channels a person is told by, under their legend. */
export const fieldLegendDefault = defineExample({
    of: FieldLegend,
    name: "default",
    description: t`The channels a person is told by, under their legend.`,
    render: (properties) => (
        <FieldSet>
            <FieldLegend {...properties}>Tell me by</FieldLegend>
            <Field orientation="horizontal">
                <Checkbox defaultChecked />
                <FieldLabel>Mail</FieldLabel>
            </Field>
            <Field orientation="horizontal">
                <Checkbox />
                <FieldLabel>Push</FieldLabel>
            </Field>
        </FieldSet>
    ),
});
