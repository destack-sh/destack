import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import {
    Field,
    FieldContent,
    FieldDescription,
    FieldError,
    FieldGroup,
    FieldLabel,
    FieldLegend,
    FieldSeparator,
    FieldSet,
    FieldTitle,
} from "./index.ts";
import { Checkbox } from "../checkbox/index.ts";
import { Input } from "../input/index.ts";
import { Select, SelectItem } from "../select/index.ts";
import { Textarea } from "../textarea/index.ts";
import { classes, markup, render } from "@destack/view/test";

/** Return the class attribute of each input and select of a container, in order. */
function controlClasses(container: Element): string[] {
    return [...container.querySelectorAll("input, select")].map((control) => control.className);
}

test("connect the label and description to the control by id", () => {
    const { container } = render(() => (
        <Field>
            <FieldLabel>Email</FieldLabel>
            <Input type="email" />
            <FieldDescription>Used to sign in.</FieldDescription>
        </Field>
    ));

    // the label points at the control, and the control names its description
    expect(markup(container)).toBe(
        '<div role="group" data-slot="field" data-orientation="vertical">' +
            '<label data-slot="field-label" for="id-1">Email</label>' +
            '<input data-slot="input" id="id-1" type="email" aria-describedby="id-2">' +
            '<p id="id-2" data-slot="field-description">Used to sign in.</p></div>',
    );
});

test("mark an invalid field's control and describe it by its error", () => {
    const [error, setError] = createSignal<string | undefined>(undefined);
    const { container } = render(() => (
        <Field invalid={error() !== undefined}>
            <FieldLabel>Title</FieldLabel>
            <Textarea />
            <FieldError errors={[{ message: error() }]} />
        </Field>
    ));
    const valid = markup(container);
    const validClasses = classes(container);
    setError("enter a title");
    flush();

    // without errors the error renders nothing and the control names no description
    expect(valid).toBe(
        '<div role="group" data-slot="field" data-orientation="vertical">' +
            '<label data-slot="field-label" for="id-1">Title</label>' +
            '<textarea data-slot="textarea" id="id-1"></textarea></div>',
    );

    // with an error the field, control and alert change together
    expect(markup(container)).toBe(
        '<div role="group" data-slot="field" data-orientation="vertical" data-invalid="true">' +
            '<label data-slot="field-label" for="id-1">Title</label>' +
            '<textarea data-slot="textarea" id="id-1" aria-invalid="true" aria-describedby="id-2"></textarea>' +
            '<div id="id-2" role="alert" data-slot="field-error">enter a title</div></div>',
    );
    expect(classes(container)).not.toEqual(validClasses);
});

test("drop a cleared error from the control's descriptions", () => {
    const [error, setError] = createSignal<string | undefined>("enter a title");
    const { container } = render(() => (
        <Field invalid={error() !== undefined}>
            <Input />
            <FieldDescription>Shown on the page.</FieldDescription>
            <FieldError errors={[{ message: error() }]} />
        </Field>
    ));
    setError(undefined);
    flush();

    // the description stays and the error leaves
    expect(markup(container)).toBe(
        '<div role="group" data-slot="field" data-orientation="vertical">' +
            '<input data-slot="input" id="id-1" aria-describedby="id-2">' +
            '<p id="id-2" data-slot="field-description">Shown on the page.</p></div>',
    );
});

test("list several errors once each, and children before errors", () => {
    const { container } = render(() => (
        <>
            <FieldError
                errors={[
                    { message: "too short" },
                    undefined,
                    { message: "too short" },
                    { message: "no digit" },
                ]}
            />
            <FieldError errors={[{ message: "too short" }]}>try another password</FieldError>
        </>
    ));
    expect(markup(container)).toBe(
        '<div id="id-1" role="alert" data-slot="field-error"><ul><li>too short</li><li>no digit</li></ul></div>' +
            '<div id="id-2" role="alert" data-slot="field-error">try another password</div>',
    );
});

test("disable a disabled field's control", () => {
    const { container } = render(() => (
        <Field disabled>
            <FieldLabel>Name</FieldLabel>
            <Input />
        </Field>
    ));
    expect(markup(container)).toBe(
        '<div role="group" data-slot="field" data-orientation="vertical" data-disabled="true">' +
            '<label data-slot="field-label" for="id-1">Name</label>' +
            '<input data-slot="input" id="id-1" disabled=""></div>',
    );
});

test("style an invalid field's controls as controls marked invalid themselves", () => {
    // draw each control in an invalid field and marked invalid outside a field
    const { container: inField } = render(() => (
        <>
            <Field invalid>
                <Input />
            </Field>
            <Field invalid>
                <Checkbox />
            </Field>
            <Field invalid>
                <Select>
                    <SelectItem value="a">A</SelectItem>
                </Select>
            </Field>
        </>
    ));
    const { container: marked } = render(() => (
        <>
            <Input aria-invalid="true" />
            <Checkbox aria-invalid="true" />
            <Select aria-invalid="true">
                <SelectItem value="a">A</SelectItem>
            </Select>
        </>
    ));
    expect(controlClasses(inField)).toEqual(controlClasses(marked));
});

test("prefer a control's attributes over its field's", () => {
    const { container } = render(() => (
        <Field>
            <Input id="name" aria-describedby="hint" />
        </Field>
    ));
    expect(markup(container)).toBe(
        '<div role="group" data-slot="field" data-orientation="vertical">' +
            '<input data-slot="input" id="name" aria-describedby="hint"></div>',
    );
});

test("render a fieldset of grouped fields with a legend, title and separator", () => {
    const { container } = render(() => (
        <FieldSet>
            <FieldLegend variant="label">Notifications</FieldLegend>
            <FieldGroup>
                <Field orientation="horizontal">
                    <FieldContent>
                        <FieldTitle>Mentions</FieldTitle>
                    </FieldContent>
                </Field>
                <FieldSeparator>or</FieldSeparator>
                <FieldSeparator />
            </FieldGroup>
        </FieldSet>
    ));
    expect(markup(container)).toBe(
        '<fieldset data-slot="field-set"><legend data-slot="field-legend" data-variant="label">Notifications</legend>' +
            '<div data-slot="field-group">' +
            '<div role="group" data-slot="field" data-orientation="horizontal"><div data-slot="field-content">' +
            '<div data-slot="field-label">Mentions</div></div></div>' +
            '<div data-slot="field-separator" data-content="true">' +
            '<div data-slot="separator" data-orientation="horizontal" role="none"></div>' +
            '<span data-slot="field-separator-content">or</span></div>' +
            '<div data-slot="field-separator" data-content="false">' +
            '<div data-slot="separator" data-orientation="horizontal" role="none"></div></div>' +
            "</div></fieldset>",
    );
});
