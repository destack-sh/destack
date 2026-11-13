import * as style from "@destack/style";
import { type Localization, t } from "@destack/locale";
import { type JsonObject, type JsonValue, schema, toJsonSchema } from "@destack/schema";
import { text } from "@destack/theme/text";
import { space } from "@destack/theme/tokens.stylex";
import { Card, CardContent, CardHeader, CardTitle } from "@destack/ui/card";
import { ItemContent, ItemDescription, ItemTitle, itemVariants } from "@destack/ui/item";
import { SchemaForm } from "@destack/ui/schema-form";
import { UniverseOrigin } from "@destack/account/address";
import { For, Loading, Show, useHome, useLocale, useQuery, useView } from "@destack/view";
import { setting } from "@destack/setting/object";
import { PERSON_SETTINGS } from "../setting/index.ts";

/** A JSON Schema as a JSON object. */
const JsonSchemaObject = schema.record(schema.string(), schema.json());

/** The settings layout: one card per setting. */
const styles = style.create({
    page: {
        display: "grid",
        gap: space[5],
        maxWidth: "48rem",
        marginInline: "auto",
        padding: space[6],
    },
});

/** Show the person's settings, each saved in their own scope. */
export default function Settings() {
    const locale = useLocale();
    const apps = UniverseOrigin.page(useView().universe, "apps").href;

    return (
        <main {...style.attrs(styles.page)}>
            {/* Title */}
            <h1 {...style.attrs(text["3xl"])}>{locale.render(t`Settings`)}</h1>

            {/* Apps with access, kept at the universe */}
            <a href={apps} {...style.attrs(itemVariants({ variant: "outline" }))}>
                <ItemContent>
                    <ItemTitle>{locale.render(t`Apps`)}</ItemTitle>
                    <ItemDescription>
                        {locale.render(t`Apps that can act for you, and what you allowed them.`)}
                    </ItemDescription>
                </ItemContent>
            </a>

            {/* Settings */}
            <Loading>
                <For each={PERSON_SETTINGS}>
                    {(declared) => <SettingCard declared={declared} />}
                </For>
            </Loading>
        </main>
    );
}

/** Edit one setting's value in the person's own scope, set over its default. */
function SettingCard(properties: {
    /** The declared setting. */
    declared: (typeof PERSON_SETTINGS)[number];
}) {
    // follow the value the person set in their own scope, copied into their home
    const locale = useLocale();
    const person = useView().user.id;
    const home = useHome({ setting });
    const reference = properties.declared.reference;
    const values = useQuery(
        home.query.setting.findMany({
            where: { ...reference, scope: person, mode: "set", package: { isNull: true } },
        }),
    );
    const current = (): JsonValue =>
        values()[0]?.value ?? schema.json().parse(properties.declared.definition.default);

    // write the value in the person's scope, creating it over the default the first time
    const save = (value: JsonValue) => {
        const [placed] = values();
        const written = { value, release: properties.declared.package.version };

        // set the first value in the person's own scope, which their scope's home stores
        if (placed === undefined) {
            home.call(setting, "create", { ...reference, ...written, scope: person, mode: "set" });
        }
        // change the value set before
        else {
            home.mutate.setting.update({ id: placed.id, ...written });
        }
    };

    return (
        <Card>
            <CardHeader>
                <CardTitle>{properties.declared.definition.title}</CardTitle>
            </CardHeader>
            <CardContent>
                {/* Value, its form rebuilt from each saved value as it arrives */}
                <Show when={JSON.stringify(current())} keyed>
                    {(saved) => (
                        <SchemaForm
                            schema={formOf(
                                properties.declared,
                                schema.json().parse(JSON.parse(saved)),
                                locale,
                            )}
                            submit={locale.render(t`Save`)}
                            onSubmit={(form) => save(valueOf(form))}
                        />
                    )}
                </Show>
            </CardContent>
        </Card>
    );
}

/** Read a submitted setting form's value, refusing a form without one. */
function valueOf(form: JsonObject): JsonValue {
    const value = form["value"];
    if (value === undefined) {
        throw new TypeError("the setting form submitted no value");
    }

    return value;
}

/** Describe the form of a setting's value, starting from its current value. */
function formOf(
    declared: (typeof PERSON_SETTINGS)[number],
    current: JsonValue,
    locale: Localization,
): JsonObject {
    // describe the value as the form's one property with the setting's description
    const described = JsonSchemaObject.parse(
        toJsonSchema(
            schema.object({
                value: declared.definition.schema.meta({
                    title: locale.render(declared.definition.title),
                    description: locale.render(declared.definition.description),
                }),
            }),
        ),
    );

    // start the property from the current value
    const properties = JsonSchemaObject.parse(described["properties"]);
    const value = JsonSchemaObject.parse(properties["value"]);

    return { ...described, properties: { value: { ...value, default: current } } };
}
