import { type JsonValue, schema } from "@destack/schema";

/** A node's name in a world. */
const NodeName = schema.string().min(1);

/** The steps every world shares, which its simulation takes, beside a driver's steps. */
export const WorldStep = schema.discriminatedUnion("action", [
    schema.object({
        /** Run the world for some milliseconds, every timer firing on time. */
        action: schema.literal("run"),
        /** The milliseconds. */
        milliseconds: schema.number().nonnegative(),
    }),
    schema.object({
        /** Move the world's clock some milliseconds at once, as a sleeping laptop wakes. */
        action: schema.literal("jump"),
        /** The milliseconds. */
        milliseconds: schema.number().nonnegative(),
    }),
    schema.object({
        /** Cut the network between two nodes, both ways. */
        action: schema.literal("partition"),
        /** The two nodes. */
        nodes: schema.tuple([NodeName, NodeName]),
    }),
    schema.object({
        /** Join two nodes again, or every pair without nodes. */
        action: schema.literal("heal"),
        /** The two nodes, every pair when absent. */
        nodes: schema.tuple([NodeName, NodeName]).exactOptional(),
    }),
    schema.object({
        /** Cut a node off from every other, as a device going offline. */
        action: schema.literal("isolate"),
        /** The node. */
        node: NodeName,
    }),
    schema.object({
        /** Join an isolated node to the network again. */
        action: schema.literal("rejoin"),
        /** The node. */
        node: NodeName,
    }),
    schema.object({
        /** Crash a node, ending its work and breaking its connections. */
        action: schema.literal("crash"),
        /** The node. */
        node: NodeName,
    }),
    schema.object({
        /** Set the faults every delivery meets. */
        action: schema.literal("degrade"),
        /** The share of deliveries lost, from 0 to 1. */
        drop: schema.number().min(0).max(1).exactOptional(),
        /** The latency of a delivery, from its least to its most milliseconds. */
        delay: schema
            .object({ least: schema.number().nonnegative(), most: schema.number().nonnegative() })
            .exactOptional(),
    }),
    schema.object({
        /** Jump a node's clock, back when negative, as a clock fault does. */
        action: schema.literal("bump"),
        /** The node. */
        node: NodeName,
        /** The milliseconds. */
        milliseconds: schema.number(),
    }),
]);
/** A step every world shares. */
export type WorldStep = schema.Infer<typeof WorldStep>;

/** The actions of the shared steps, which a driver hands to the simulation. */
const WORLD_ACTIONS: ReadonlySet<string> = new Set(
    WorldStep.options.map((option) => option.shape.action.value),
);

/** A world as data: the seed and start its simulation runs from, and the steps it takes. */
export interface World<Step = JsonValue> {
    /** The seed, which reproduces the world. */
    readonly seed: number;
    /** The start, in UTC epoch milliseconds. */
    readonly start: number;
    /** The steps, the shared ones and a driver's, taken in order. */
    readonly steps: readonly (WorldStep | Step)[];
}

/** Worlds. */
export const World = {
    /** Report whether a step is one every world shares, which the simulation takes itself. */
    isStep(step: unknown): step is WorldStep {
        return (
            typeof step === "object" &&
            step !== null &&
            "action" in step &&
            typeof step.action === "string" &&
            WORLD_ACTIONS.has(step.action)
        );
    },

    /** Read a world's first steps, as a scrubbed replay plays them. */
    scrub<Step>(world: World<Step>, steps: number): World<Step> {
        return { ...world, steps: world.steps.slice(0, steps) };
    },

    /** Branch a world after its first steps into new ones. */
    branch<Step>(
        world: World<Step>,
        after: number,
        steps: readonly (WorldStep | Step)[],
    ): World<Step> {
        return { ...world, steps: [...world.steps.slice(0, after), ...steps] };
    },
};
