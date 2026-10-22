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

/** The runtime the toolchain runs on: Bun, which evaluates declarations and loads npm units. */
export const HOST_RUNTIME = "bun" satisfies Runtime;

/** The runtimes resolving imports from deployed files: Bun, unlike workerd's module list. */
export const FILE_RESOLVING_RUNTIMES: ReadonlySet<Runtime> = new Set(["bun"]);

/** The runtimes loading native addons: Bun, as workerd runs no native code. */
export const NATIVE_ADDON_RUNTIMES: ReadonlySet<Runtime> = new Set(["bun"]);

/** The scheme of the modules workerd supplies itself, such as `cloudflare:sockets`. */
export const WORKERD_SCHEME = /^cloudflare:/u;

/** The runtimes whose program reads a package's whole source, in preference: the host's first. */
export const PRIMARY_RUNTIMES: readonly Runtime[] = [HOST_RUNTIME, "workerd", "browser"];
