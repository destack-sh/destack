import { plugin } from "bun";
import { present } from "@destack/schema";
import type { Kernel } from "../computer/kernel.ts";

/** The link of each kernel specifier, process-wide as Bun's module registry is. */
const LINKS = new Map<string, Promise<void>>();

/** Link the specifiers of the packages a kernel provides to this process's own instances. */
export async function linkKernel(specifiers: readonly string[], kernel: Kernel): Promise<void> {
    // link each provided specifier not linked yet, then wait for every link the caller needs, in flight ones included
    const needed = [...new Set(specifiers)].filter((specifier) => kernel.provides(specifier));
    for (const specifier of needed.filter((each) => !LINKS.has(each))) {
        LINKS.set(specifier, link(specifier));
    }
    await Promise.all(
        needed.map((specifier) => present(LINKS.get(specifier), `the link of ${specifier}`)),
    );
}

/** Register a specifier as a virtual module answering this process's own instance. */
async function link(specifier: string): Promise<void> {
    // read the module this process resolves the specifier to, before registering, as a registered specifier answers itself
    const path = Bun.resolveSync(specifier, import.meta.dirname);
    const exports: unknown = await import(path);
    if (typeof exports !== "object" || exports === null) {
        throw new TypeError(`the kernel module ${specifier} at ${path} exports nothing`);
    }
    const namespace = { ...exports };

    // register it as a virtual module answering that instance
    plugin({
        name: `destack-kernel ${specifier}`,
        setup(build) {
            build.module(specifier, () => ({ exports: namespace, loader: "object" }));
        },
    });
}
