import { isServer } from "@solidjs/web";
import { createRoot, getOwner, onCleanup, type Owner, runWithOwner, sharedConfig } from "solid-js";

/** Run a computation in a root of its own, disposed with any of its owners, the current one by default. */
export function createSubRoot<Result>(
    compute: (dispose: () => void) => Result,
    ...owners: (Owner | null)[]
): Result {
    // dispose the root with each owner
    const parents = owners.length === 0 ? [getOwner()] : owners;

    return runWithOwner(parents[0] ?? null, () =>
        createRoot((dispose) => {
            for (const owner of parents) {
                if (owner !== null) {
                    runWithOwner(owner, () => onCleanup(dispose));
                }
            }

            return compute(dispose);
        }),
    );
}

/** Share one root between its users, created on first use and disposed after the last user is. */
export function createSingletonRoot<Value>(
    factory: (dispose: () => void) => Value,
    detachedOwner: Owner | null = getOwner(),
): () => Value {
    let users = 0;
    let current: { readonly value: Value; readonly dispose: () => void } | undefined;

    return () => {
        // count the user, disposing the root a microtask after the last one leaves
        users++;
        onCleanup(() => {
            users--;
            queueMicrotask(() => {
                if (users === 0 && current !== undefined) {
                    current.dispose();
                    current = undefined;
                }
            });
        });

        // create the root on first use
        current ??= runWithOwner(detachedOwner, () =>
            createRoot((dispose) => ({ value: factory(dispose), dispose })),
        );

        return current.value;
    };
}

/** Share one root between its users, except on the server and while hydrating, where each user gets a root of its own at its place in the hydration keys. */
export function createHydratableSingletonRoot<Value>(
    factory: (dispose: () => void) => Value,
): () => Value {
    const singleton = createSingletonRoot(factory);

    return () => (isServer || sharedConfig.hydrating ? createSubRoot(factory) : singleton());
}
