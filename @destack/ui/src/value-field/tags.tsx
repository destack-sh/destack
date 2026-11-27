import minusIcon from "@destack/icon/phosphor/minus";
import plusIcon from "@destack/icon/phosphor/plus";
import { t } from "@destack/locale";
import type { schema } from "@destack/schema";
import * as style from "@destack/style";
import { createSignal, For, type JSX, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import { Icon } from "../icon/index.ts";
import { Input } from "../input/index.ts";
import { NumberField } from "../number-field/index.ts";
import { ValueGroup, valueStyles } from "./group.tsx";

/** Tagged numbers, such as OpenType features or font axes. */
type Tags = Readonly<Record<string, number>>;

/** Render tagged numbers: each tag and its number, removed or added by tag. */
export function TagsParts(properties: {
    /** What the numbers are called. */
    readonly label: string | undefined;
    /** Whether it takes no input. */
    readonly disabled: boolean;
    /** The tagged numbers. */
    readonly value: Tags;
    /** The schema that reads the numbers, refusing a tag or number it takes not. */
    readonly schema: schema.Schema<Tags>;
    /** The number a tag starts at once added. */
    readonly start: number;
    /** The step of each number. */
    readonly step: number;
    /** Handle the numbers changing to ones the schema takes. */
    readonly onValueChange: (value: Tags) => void;
}): JSX.Element {
    // give back numbers the schema takes, keeping each tag in its place
    const locale = useLocale();
    const [added, setAdded] = createSignal("");
    const entries = (): readonly (readonly [string, number])[] => Object.entries(properties.value);
    const write = (next: readonly (readonly [string, number])[]): void => {
        const read = properties.schema.safeParse(Object.fromEntries(next));
        if (read.success) {
            properties.onValueChange(read.data);
        }
    };

    return (
        <ValueGroup aria-label={properties.label} xstyle={valueStyles.group}>
            {/* Each tag and its number, in place as they change */}
            <For each={entries()} keyed={false}>
                {(entry, index) => (
                    <div {...style.attrs(valueStyles.row)}>
                        <Input
                            aria-label={locale.render(t.context("label")`Tag`)}
                            disabled={properties.disabled}
                            value={entry()[0]}
                            onChange={(event) =>
                                write(
                                    entries().with(index, [event.currentTarget.value, entry()[1]]),
                                )
                            }
                        />
                        <NumberField
                            aria-label={entry()[0]}
                            step={properties.step}
                            disabled={properties.disabled}
                            value={entry()[1]}
                            onValueChange={(next) =>
                                next !== undefined &&
                                write(entries().with(index, [entry()[0], next]))
                            }
                        />
                        <Button
                            variant="ghost"
                            isIconOnly
                            disabled={properties.disabled}
                            aria-label={locale.render(t.context("action")`Remove ${entry()[0]}`)}
                            onClick={() => write(entries().toSpliced(index, 1))}
                        >
                            <Icon icon={minusIcon} />
                        </Button>
                    </div>
                )}
            </For>

            {/* A tag to add */}
            <div {...style.attrs(valueStyles.row)}>
                <Input
                    aria-label={locale.render(t.context("label")`New tag`)}
                    disabled={properties.disabled}
                    value={added()}
                    onInput={(event) => setAdded(event.currentTarget.value)}
                />
                <Button
                    variant="ghost"
                    disabled={properties.disabled}
                    onClick={() => {
                        write([...entries(), [added(), properties.start]]);
                        setAdded("");
                    }}
                >
                    <Icon icon={plusIcon} />
                    {locale.render(t.context("action")`Add`)}
                </Button>
            </div>
        </ValueGroup>
    );
}
