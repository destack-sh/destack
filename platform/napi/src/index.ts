import * as generated from "./generated";

export * from "./generated";

export type RunOptions = Omit<
    generated.RunOptions,
    "targetOverridesJson" | "runtimeOverridesJson"
> & {
    targetOverrides?: unknown;
    runtimeOverrides?: unknown;
    targetOverridesJson?: string;
    runtimeOverridesJson?: string;
};

export type RunInput = generated.RunInput | string;

function normalizeRunOptions(options?: RunOptions | null): generated.RunOptions | undefined {
    if (options == null) {
        return undefined;
    }

    const next = { ...options } as RunOptions & {
        targetOverrides?: unknown;
        runtimeOverrides?: unknown;
    };

    if (next.targetOverrides !== undefined && next.targetOverridesJson === undefined) {
        next.targetOverridesJson = JSON.stringify(next.targetOverrides);
    }
    if (next.runtimeOverrides !== undefined && next.runtimeOverridesJson === undefined) {
        next.runtimeOverridesJson = JSON.stringify(next.runtimeOverrides);
    }

    delete next.targetOverrides;
    delete next.runtimeOverrides;

    return next as generated.RunOptions;
}

function normalizeRunInput(input: RunInput): generated.RunInput {
    if (typeof input === "string") {
        return {
            kind: generated.RunInputKind.Inline,
            name: "<eval>.ds",
            content: input,
        };
    }

    return input;
}

function normalizeInlineRunOptions(options?: RunOptions | null): RunOptions | undefined {
    if (options == null) {
        return {
            ...generated.defaultRunOptions(),
            mode: generated.RunMode.EvalPrint,
        };
    }

    if (options.mode !== undefined) {
        return options;
    }

    return {
        ...options,
        mode: generated.RunMode.EvalPrint,
    };
}

export function runSync(input: RunInput, options?: RunOptions | null): generated.RunResult {
    const nextInput = normalizeRunInput(input);
    const nextOptions =
        typeof input === "string"
            ? normalizeRunOptions(normalizeInlineRunOptions(options))
            : normalizeRunOptions(options);
    return generated.runSync(nextInput, nextOptions);
}

export function runEvalSync(code: string, options?: RunOptions | null): generated.RunResult {
    const nextOptions = normalizeRunOptions(normalizeInlineRunOptions(options));
    return generated.runSync(
        {
            kind: generated.RunInputKind.Inline,
            name: "<eval>.ds",
            content: code,
        },
        nextOptions,
    );
}
