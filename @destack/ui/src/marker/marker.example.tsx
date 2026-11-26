import { t } from "@destack/locale";
import arrowSquareOutIcon from "@destack/icon/phosphor/arrow-square-out";
import brainIcon from "@destack/icon/phosphor/brain";
import checkCircleIcon from "@destack/icon/phosphor/check-circle";
import circleDashedIcon from "@destack/icon/phosphor/circle-dashed";
import fileTextIcon from "@destack/icon/phosphor/file-text";
import globeIcon from "@destack/icon/phosphor/globe";
import magnifyingGlassIcon from "@destack/icon/phosphor/magnifying-glass";
import robotIcon from "@destack/icon/phosphor/robot";
import terminalWindowIcon from "@destack/icon/phosphor/terminal-window";
import warningCircleIcon from "@destack/icon/phosphor/warning-circle";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { motion, space } from "@destack/theme/tokens.stylex";
import {
    createSignal,
    For,
    type JSX,
    onCleanup,
    onSettled,
    Show,
    untrack,
    useEnvironment,
} from "@destack/view";
import { Button } from "../button/index.ts";
import { CodeBlock, CodeBlockContent } from "../code-block/index.ts";
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleIndicator,
    CollapsibleTrigger,
} from "../collapsible/index.ts";
import { Icon } from "../icon/index.ts";
import { Spinner } from "../spinner/index.ts";
import { Stack } from "../stack/index.ts";
import {
    Marker,
    MarkerContent,
    MarkerDescription,
    MarkerIcon,
    type MarkerStatus,
} from "./marker.tsx";

/** The pages an answer uses. */
const SOURCES = [
    ["Opening hours of the museum", "https://example.com/hours"],
    ["Trains on public holidays", "https://example.com/trains"],
] as const;

/** The steps of a plan, and how far each has come. */
const PLAN = [
    ["Read the notes of March", "done"],
    ["Compare the quotes", "running"],
    ["Write the summary", "open"],
] as const;

/** How long each step of the live example takes, in milliseconds. */
const STEP = 1_400;

/** The steps of the live example, the last ones those of a second agent. */
const LIVE = ["Read the notes of March", "Compare the quotes", "Read quotes.pdf", "Read terms.pdf"];

/** The styles of the examples. */
const styles = style.create({
    // fade each step in as it arrives
    arriving: {
        opacity: { default: 1, "@starting-style": 0 },
        translate: { default: "none", "@starting-style": `0 calc(-1 * ${space[1]})` },
        transitionProperty: "opacity, translate",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
    },
});

/** Render a step whose detail shows below it. */
function Step(properties: {
    /** The step's icon. */
    readonly icon: JSX.Element;
    /** What it does or did. */
    readonly title: string;
    /** A count, or how long it took. */
    readonly description?: string;
    /** Its status, none once done. */
    readonly status?: MarkerStatus;
    /** Whether the detail shows at first. */
    readonly isOpen?: boolean;
    /** The detail. */
    readonly children: JSX.Element;
}): JSX.Element {
    return (
        <Collapsible defaultOpen={properties.isOpen === true}>
            <Marker
                status={properties.status}
                render={(attributes) => <CollapsibleTrigger {...attributes} />}
            >
                <MarkerIcon>{properties.icon}</MarkerIcon>
                <MarkerContent>{properties.title}</MarkerContent>
                <MarkerDescription>{properties.description}</MarkerDescription>
                <CollapsibleIndicator />
            </Marker>
            <CollapsibleContent variant="line">{properties.children}</CollapsibleContent>
        </Collapsible>
    );
}

/** What an assistant did for an answer, and does now. */
export const markerSteps = defineExample({
    of: Marker,
    name: "steps",
    description: t`What an assistant did for an answer, the last step under way.`,
    layout: "padded",
    render: () => (
        <Stack space="3">
            <Marker>
                <MarkerIcon>
                    <Icon icon={magnifyingGlassIcon} />
                </MarkerIcon>
                <MarkerContent>Searched the notes of March</MarkerContent>
            </Marker>
            <Marker render={(attributes) => <a {...attributes} href="#budget" />}>
                <MarkerIcon>
                    <Icon icon={fileTextIcon} />
                </MarkerIcon>
                <MarkerContent>Read Budget for the spring trip</MarkerContent>
            </Marker>
            <Marker role="status" status="running">
                <MarkerIcon>
                    <Spinner />
                </MarkerIcon>
                <MarkerContent>Comparing the quotes</MarkerContent>
            </Marker>
        </Stack>
    ),
});

/** What an agent thought, ran and used, each with its detail. */
export const markerAgent = defineExample({
    of: Marker,
    name: "agent",
    description: t`What an agent thought, ran, failed to run and used, each with its detail.`,
    layout: "padded",
    render: () => (
        <Stack space="2">
            <Step icon={<Icon icon={brainIcon} />} title="Thought" description="5s">
                <Marker>The quotes arrive in three currencies, so they need one rate.</Marker>
            </Step>
            <Step
                icon={<Icon icon={terminalWindowIcon} />}
                title="Ran bun test"
                description="1.2s"
                isOpen
            >
                <CodeBlock>
                    <CodeBlockContent code={"$ bun test\n 14 pass\n 0 fail"} label="bun test" />
                </CodeBlock>
            </Step>
            <Step
                icon={<Icon icon={warningCircleIcon} />}
                title="Failed to fetch the exchange rates"
                status="error"
            >
                <CodeBlock>
                    <CodeBlockContent code="503 Service Unavailable" label="error" />
                </CodeBlock>
            </Step>
            <Step icon={<Icon icon={globeIcon} />} title="Used 2 sources">
                <Stack space="2">
                    <For each={SOURCES}>
                        {([title, address]) => (
                            <Marker
                                render={(attributes) => (
                                    <a
                                        {...attributes}
                                        href={address}
                                        target="_blank"
                                        rel="noreferrer"
                                    />
                                )}
                            >
                                <MarkerContent>{title}</MarkerContent>
                                <MarkerIcon>
                                    <Icon icon={arrowSquareOutIcon} />
                                </MarkerIcon>
                            </Marker>
                        )}
                    </For>
                </Stack>
            </Step>
            <Step
                icon={<Icon icon={robotIcon} />}
                title="Comparing the quotes in a second agent"
                description="12s"
                status="running"
                isOpen
            >
                <Stack space="2">
                    <Step icon={<Icon icon={fileTextIcon} />} title="Read quotes.pdf" isOpen>
                        <Step
                            icon={<Icon icon={magnifyingGlassIcon} />}
                            title="Found 3 prices"
                            isOpen
                        >
                            <Marker>€4,200, £3,900 and CHF 4,050, each without tax.</Marker>
                        </Step>
                    </Step>
                    <Marker status="running">
                        <MarkerIcon>
                            <Spinner />
                        </MarkerIcon>
                        <MarkerContent>Reading terms.pdf</MarkerContent>
                    </Marker>
                </Stack>
            </Step>
        </Stack>
    ),
});

/** The steps an agent plans, and how far it has come. */
export const markerPlan = defineExample({
    of: Marker,
    name: "plan",
    description: t`The steps an agent plans, and how far it has come.`,
    layout: "padded",
    render: () => (
        <Step icon={<Icon icon={magnifyingGlassIcon} />} title="Plan" description="1 of 3" isOpen>
            <Stack space="2">
                <For each={PLAN}>
                    {([title, state]) => (
                        <Marker status={state === "running" ? "running" : undefined}>
                            <MarkerIcon>
                                {state === "running" ? (
                                    <Spinner />
                                ) : (
                                    <Icon
                                        icon={state === "done" ? checkCircleIcon : circleDashedIcon}
                                    />
                                )}
                            </MarkerIcon>
                            <MarkerContent>{title}</MarkerContent>
                        </Marker>
                    )}
                </For>
            </Stack>
        </Step>
    ),
});

/** An agent's turn as it happens: open while it works, closed to a summary once done. */
export const markerLive = defineExample({
    of: Marker,
    name: "live",
    description: t`An agent's turn as it happens: open while it works, and a summary once done.`,
    layout: "padded",
    render: () => {
        // take a step at a time
        const { clock } = useEnvironment();
        const [done, setDone] = createSignal(0, { ownedWrite: true });
        const [chosen, setChosen] = createSignal<boolean>();
        let timer: number | undefined;
        const advance = (): void => {
            timer = clock.setTimeout(() => {
                setDone((count) => count + 1);
                if (untrack(done) + 1 < LIVE.length) {
                    advance();
                }
            }, STEP);
        };
        const replay = (): void => {
            // start over, following the work again
            clock.clearTimeout(timer);
            setDone(0);
            setChosen(undefined);
            advance();
        };
        onSettled(replay);
        onCleanup(() => clock.clearTimeout(timer));

        // follow the work until the person opens or closes it
        const isWorking = (): boolean => done() < LIVE.length;
        const isOpen = (): boolean => chosen() ?? isWorking();
        const status = (index: number): MarkerStatus | undefined =>
            done() === index ? "running" : undefined;
        const Row = (properties: { readonly index: number }): JSX.Element => (
            <Show when={done() >= properties.index}>
                <Marker status={status(properties.index)} xstyle={styles.arriving}>
                    <MarkerIcon>
                        <Show when={done() > properties.index} fallback={<Spinner />}>
                            <Icon icon={checkCircleIcon} />
                        </Show>
                    </MarkerIcon>
                    <MarkerContent>{LIVE[properties.index]}</MarkerContent>
                </Marker>
            </Show>
        );

        return (
            <Stack space="4">
                <Collapsible open={isOpen()} onOpenChange={(next) => void setChosen(next)}>
                    <Marker
                        status={isWorking() ? "running" : undefined}
                        render={(attributes) => <CollapsibleTrigger {...attributes} />}
                    >
                        <MarkerIcon>
                            <Show when={isWorking()} fallback={<Icon icon={brainIcon} />}>
                                <Spinner />
                            </Show>
                        </MarkerIcon>
                        <MarkerContent>{isWorking() ? "Working" : "Worked"}</MarkerContent>
                        <MarkerDescription>
                            {Math.min(done(), LIVE.length)} of {LIVE.length}
                        </MarkerDescription>
                        <CollapsibleIndicator />
                    </Marker>
                    <CollapsibleContent variant="line">
                        <Stack space="2">
                            <Row index={0} />
                            <Row index={1} />
                            <Show when={done() >= 2}>
                                <Collapsible defaultOpen xstyle={styles.arriving}>
                                    <Marker
                                        status={isWorking() ? "running" : undefined}
                                        render={(attributes) => (
                                            <CollapsibleTrigger {...attributes} />
                                        )}
                                    >
                                        <MarkerIcon>
                                            <Icon icon={robotIcon} />
                                        </MarkerIcon>
                                        <MarkerContent>Asked a second agent</MarkerContent>
                                        <CollapsibleIndicator />
                                    </Marker>
                                    <CollapsibleContent variant="line">
                                        <Stack space="2">
                                            <Row index={2} />
                                            <Row index={3} />
                                        </Stack>
                                    </CollapsibleContent>
                                </Collapsible>
                            </Show>
                        </Stack>
                    </CollapsibleContent>
                </Collapsible>
                <div>
                    <Button variant="outline" size="sm" onClick={replay}>
                        Replay
                    </Button>
                </div>
            </Stack>
        );
    },
});

/** A day, someone joining, and the end of a section. */
export const markerSeparators = defineExample({
    of: Marker,
    name: "separators",
    description: t`A day and someone joining between two lines, and a row above a line.`,
    layout: "padded",
    render: () => (
        <Stack space="4">
            <Marker variant="separator">
                <MarkerContent>Today</MarkerContent>
            </Marker>
            <Marker variant="separator">
                <MarkerContent>Grace joined the conversation</MarkerContent>
            </Marker>
            <Marker variant="border">
                <MarkerContent>Ada renamed the channel to launch</MarkerContent>
            </Marker>
        </Stack>
    ),
});
