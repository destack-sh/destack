import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { Icon } from "../icon/index.ts";
import { Button } from "../button/index.ts";
import delayIcon from "@destack/icon/phosphor/hourglass";
import plusIcon from "@destack/icon/phosphor/plus";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import { present } from "@destack/schema";
import {
    Motion,
    type MotionAnimation,
    MOTION_STATES,
    MotionState,
    Stagger,
    TimingFunction,
    Transition,
    ViewTransitionName,
} from "@destack/style";
import * as style from "@destack/style";
import { createSignal, type JSX, Show, untrack, useLocale } from "@destack/view";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";
import {
    choiceType,
    jsonType,
    ValueNone,
    ValuePart,
    ValueSet,
    ValueStack,
    valueStyles,
} from "../value-field/group.tsx";
import { Field, FieldError, FieldGroup, FieldLabel } from "../field/index.ts";
import { Input } from "../input/index.ts";
import { TimePart } from "../value-field/transition.tsx";
import { AnimationField, AnimationSummary } from "./animation.tsx";
import { FramePopover } from "./frame.tsx";
import { MotionStage } from "./stage.tsx";

/** The transition each part of a new motion starts with: quick, along CSS's ease. */
const NEW_TRANSITION: Transition = {
    duration: { milliseconds: 200 },
    delay: { milliseconds: 0 },
    timingFunction: TimingFunction.initial,
};

/** The motion a person adds: a quick standard transition, without states or animations. */
const NEW_MOTION: Motion = {
    transition: NEW_TRANSITION,
    states: [],
    enter: { values: [], transition: NEW_TRANSITION },
    exit: { values: [], transition: NEW_TRANSITION },
    animations: [],
    stagger: { delay: { milliseconds: 0 }, from: "first" },
    viewTransition: "none",
};

/** The animation a person adds: one second fading in, on time. */
const NEW_ANIMATION: MotionAnimation = {
    tracks: [
        {
            property: "opacity",
            keyframes: [
                { offset: 0, value: 0, easing: TimingFunction.initial },
                { offset: 1, value: 1, easing: TimingFunction.initial },
            ],
        },
    ],
    duration: { milliseconds: 1000 },
    delay: { milliseconds: 0 },
    iterations: 1,
    direction: "normal",
    fill: "both",
    timeline: { kind: "time" },
};

/** The types of a motion's parts. */
const TYPES = {
    transition: jsonType(Transition),
    state: choiceType(MotionState.options, MOTION_STATES),
    from: choiceType(Stagger.shape.from.options, {
        first: { title: t.context("option")`First child` },
        last: { title: t.context("option")`Last child` },
    }),
};

/** The styles of a motion field. */
const styles = style.create({
    picker: {
        width: "18rem",
    },
});

/** The properties people change in a motion field's controls and inspector. */
const MOTION_FIELD_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses changes where the motion cannot change now.`)
        .default(false),
};

/** The properties of a motion field besides its declared ones: its motion. */
export interface MotionFieldAttributes {
    /** What the motion is called, shown as its legend. */
    readonly label?: string | undefined;
    /** What the motion is called where no legend shows. */
    readonly "aria-label"?: string | undefined;
    /** The motion, none where an element does not move. */
    readonly value: Motion | undefined;
    /** Handle a person changing the motion. */
    readonly onValueChange: (value: Motion) => void;
}

/** Render a motion's field: its stage, transition, states, entry, exit and animations. */
export const MotionField = defineComponent({
    name: "motionField",
    properties: MOTION_FIELD_PROPERTIES,
    render(
        properties: RenderProperties<MotionFieldAttributes, typeof MOTION_FIELD_PROPERTIES>,
    ): JSX.Element {
        // read the motion, and write it back changed
        const locale = useLocale();
        const change = (next: Partial<Motion>): void =>
            properties.onValueChange({
                ...present(properties.value, "a motion to change"),
                ...next,
            });

        // call each animation by its place
        const animationLabel = (index: number): string =>
            locale.render(t.context("label")`Animation ${index + 1}`);

        return (
            <Show
                when={properties.value}
                fallback={
                    <ValueNone
                        label={properties.label}
                        aria-label={properties["aria-label"]}
                        disabled={properties.disabled}
                        onAdd={() => properties.onValueChange(NEW_MOTION)}
                    />
                }
            >
                {(motion) => (
                    <ValueSet label={properties.label} aria-label={properties["aria-label"]}>
                        {/* A sample moving as the motion does */}
                        <MotionStage motion={motion()} />

                        {/* How it moves between states, enters and exits */}
                        <ValuePart
                            type={TYPES.transition}
                            schema={Transition}
                            label={t.context("label")`Transition`}
                            isLabelShown
                            value={motion().transition}
                            disabled={properties.disabled}
                            onValueChange={(transition) => change({ transition })}
                        />

                        {/* The values of each state, the next free state added */}
                        <ValueSet
                            label={locale.render(t.context("label")`States`)}
                            actions={
                                <Show
                                    when={MotionState.options.find(
                                        (state) =>
                                            !motion().states.some((each) => each.state === state),
                                    )}
                                >
                                    {(free) => (
                                        <Button
                                            variant="ghost"
                                            size="sm"
                                            isIconOnly
                                            aria-label={locale.render(
                                                t.context("action")`Add state`,
                                            )}
                                            disabled={properties.disabled}
                                            onClick={() =>
                                                change({
                                                    states: [
                                                        ...motion().states,
                                                        {
                                                            state: free(),
                                                            values: [],
                                                            transition: motion().transition,
                                                        },
                                                    ],
                                                })
                                            }
                                        >
                                            <Icon icon={plusIcon} />
                                        </Button>
                                    )}
                                </Show>
                            }
                        >
                            <ValueStack
                                items={motion().states}
                                label={(index) =>
                                    locale.render(
                                        MOTION_STATES[
                                            present(motion().states[index], "a state").state
                                        ].title,
                                    )
                                }
                                disabled={properties.disabled}
                                onValueChange={(states) => change({ states: [...states] })}
                            >
                                {(each, index) => {
                                    const title = () =>
                                        locale.render(MOTION_STATES[each().state].title);

                                    return (
                                        <>
                                            <ValuePart
                                                type={TYPES.state}
                                                schema={MotionState}
                                                label={t.context("label")`State`}
                                                value={each().state}
                                                disabled={properties.disabled}
                                                onValueChange={(state) =>
                                                    change({
                                                        states: motion().states.with(index, {
                                                            ...each(),
                                                            state,
                                                        }),
                                                    })
                                                }
                                            />
                                            <FramePopover
                                                label={title()}
                                                value={each()}
                                                variant="ghost"
                                                disabled={properties.disabled}
                                                onValueChange={(frame) =>
                                                    change({
                                                        states: motion().states.with(index, {
                                                            ...frame,
                                                            state: each().state,
                                                        }),
                                                    })
                                                }
                                            />
                                        </>
                                    );
                                }}
                            </ValueStack>
                        </ValueSet>

                        {/* The frames it enters from and exits to */}
                        <ValueSet label={locale.render(t.context("label")`Enter`)}>
                            <FramePopover
                                label={locale.render(t.context("label")`Enter`)}
                                value={motion().enter}
                                variant="outline"
                                disabled={properties.disabled}
                                onValueChange={(enter) => change({ enter })}
                            />
                        </ValueSet>
                        <ValueSet label={locale.render(t.context("label")`Exit`)}>
                            <FramePopover
                                label={locale.render(t.context("label")`Exit`)}
                                value={motion().exit}
                                variant="outline"
                                disabled={properties.disabled}
                                onValueChange={(exit) => change({ exit })}
                            />
                        </ValueSet>

                        {/* Its animations, each picked apart */}
                        <ValueSet
                            label={locale.render(t.context("label")`Animations`)}
                            actions={
                                <Button
                                    variant="ghost"
                                    size="sm"
                                    isIconOnly
                                    aria-label={locale.render(t.context("action")`Add animation`)}
                                    disabled={properties.disabled}
                                    onClick={() =>
                                        change({
                                            animations: [...motion().animations, NEW_ANIMATION],
                                        })
                                    }
                                >
                                    <Icon icon={plusIcon} />
                                </Button>
                            }
                        >
                            <ValueStack
                                items={motion().animations}
                                label={animationLabel}
                                disabled={properties.disabled}
                                onValueChange={(animations) =>
                                    change({ animations: [...animations] })
                                }
                            >
                                {(animation, index) => (
                                    <Popover>
                                        <PopoverTrigger
                                            render={(attributes) => (
                                                <Button
                                                    {...attributes}
                                                    variant="ghost"
                                                    aria-label={animationLabel(index)}
                                                    disabled={properties.disabled}
                                                    xstyle={valueStyles.summary}
                                                >
                                                    <AnimationSummary value={animation()} />
                                                </Button>
                                            )}
                                        />
                                        <PopoverContent
                                            title={animationLabel(index)}
                                            side="left"
                                            align="start"
                                            xstyle={styles.picker}
                                        >
                                            <AnimationField
                                                aria-label={animationLabel(index)}
                                                value={animation()}
                                                disabled={properties.disabled}
                                                onValueChange={(next) =>
                                                    change({
                                                        animations: motion().animations.with(
                                                            index,
                                                            next,
                                                        ),
                                                    })
                                                }
                                            />
                                        </PopoverContent>
                                    </Popover>
                                )}
                            </ValueStack>
                        </ValueSet>

                        {/* How it delays its children, one after another */}
                        <ValueSet label={locale.render(t.context("label")`Stagger`)}>
                            <FieldGroup columns={2}>
                                <TimePart
                                    label={locale.render(t.context("label")`Delay`)}
                                    icon={delayIcon}
                                    disabled={properties.disabled}
                                    value={motion().stagger.delay}
                                    onValueChange={(delay) =>
                                        change({ stagger: { ...motion().stagger, delay } })
                                    }
                                />
                                <ValuePart
                                    type={TYPES.from}
                                    schema={Stagger.shape.from}
                                    label={t.context("label")`Starts from`}
                                    value={motion().stagger.from}
                                    disabled={properties.disabled}
                                    onValueChange={(from) =>
                                        change({ stagger: { ...motion().stagger, from } })
                                    }
                                />
                            </FieldGroup>
                        </ValueSet>

                        {/* The name it morphs under between views */}
                        <ViewTransitionPart
                            value={motion().viewTransition}
                            disabled={properties.disabled}
                            onValueChange={(viewTransition) => change({ viewTransition })}
                        />
                    </ValueSet>
                )}
            </Show>
        );
    },
});

/** The properties of a motion field, its motion included. */
export type MotionFieldProperties = ComponentProperties<typeof MotionField>;

/** Render the name an element morphs under between views, empty for none, CSS identifiers only. */
function ViewTransitionPart(properties: {
    /** The name, or none. */
    readonly value: Motion["viewTransition"];
    /** Whether it takes no input. */
    readonly disabled: boolean | undefined;
    /** Handle a person giving a name CSS takes, or clearing it. */
    readonly onValueChange: (value: Motion["viewTransition"]) => void;
}): JSX.Element {
    // keep what a person types, writing it once it is empty or a name CSS takes
    const locale = useLocale();
    const [typed, setTyped] = createSignal(
        untrack(() => (properties.value === "none" ? "" : properties.value)),
    );
    const problem = (): string | undefined =>
        typed() === "" || ViewTransitionName.safeParse(typed()).success
            ? undefined
            : locale.render(t`Start with a letter, then use letters, digits, - and _`);
    const type = (text: string): void => {
        setTyped(text);
        const name = ViewTransitionName.safeParse(text).data;
        if (text === "") {
            properties.onValueChange("none");
        } else if (name !== undefined) {
            properties.onValueChange(name);
        }
    };

    return (
        <Field invalid={problem() !== undefined}>
            <FieldLabel>{locale.render(t.context("label")`View transition`)}</FieldLabel>
            <Input
                value={typed()}
                placeholder={locale.render(t.context("label")`None`)}
                disabled={properties.disabled}
                onInput={(event) => type(event.currentTarget.value)}
            />
            <FieldError errors={[{ message: problem() }]} />
        </Field>
    );
}
