import type { schema } from "@destack/schema";
import type { Setting } from "./setting.ts";
import type { SettingResolution } from "./resolution.ts";

/** Named declarations requested under one consumer and target. */
export type SettingBatch = Readonly<Record<string, Setting>>;

/** Resolutions retaining each declaration's inferred value type. */
export type SettingResult<Batch extends SettingBatch> = {
    readonly [Name in keyof Batch]: SettingResolution<
        schema.Infer<Batch[Name]["declaration"]["schema"]>
    >;
};

/** Host-bound access using verified caller and receiving-deployment context. */
export interface SettingContext {
    /** Resolve a batch under one consistent authority read. */
    resolve<Batch extends SettingBatch>(settings: Batch): Promise<SettingResult<Batch>>;
    /** Observe complete current batches until cancellation or authority expiry. */
    watch<Batch extends SettingBatch>(
        settings: Batch,
        signal: AbortSignal,
    ): AsyncIterable<SettingResult<Batch>>;
}
