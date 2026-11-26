import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Field, FieldError, FieldLabel } from "../field/index.ts";
import { PasswordField } from "./password-field.tsx";

/** A password a person gives. */
export const passwordFieldSignIn = defineExample({
    of: PasswordField,
    name: "signIn",
    description: t`A password a person gives.`,
    render: (properties) => (
        <Field>
            <FieldLabel>Password</FieldLabel>
            <PasswordField defaultValue="correct horse" {...properties} />
        </Field>
    ),
});

/** A secret that is refused. */
export const passwordFieldInvalid = defineExample({
    of: PasswordField,
    name: "invalid",
    description: t`A secret that is refused.`,
    render: (properties) => (
        <Field data-invalid="true">
            <FieldLabel>Token</FieldLabel>
            <PasswordField
                autocomplete="off"
                aria-invalid="true"
                defaultValue="token"
                {...properties}
            />
            <FieldError>The token expired.</FieldError>
        </Field>
    ),
});
