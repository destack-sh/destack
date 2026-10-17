import { schema } from "@destack/schema";

/** The runtimes hosts execute workloads in. */
export const SERVER_RUNTIMES = ["bun", "workerd"] as const;

/** The runtime selected by the compiler adapter. */
export const Runtime = schema.enum(["browser", ...SERVER_RUNTIMES]);
/** The runtime selected by the compiler adapter. */
export type Runtime = schema.Infer<typeof Runtime>;

/** A runtime that hosts execute workloads in. */
export const ServerRuntime = schema.enum(SERVER_RUNTIMES);
/** A runtime that hosts execute workloads in. */
export type ServerRuntime = schema.Infer<typeof ServerRuntime>;
