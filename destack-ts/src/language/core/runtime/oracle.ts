/**
 * The oracle for all our entropy (e.g., time, randomness).
 * Useful to isolate non-determinsim, and of course to mock in simulation testing.
 */
export abstract class Oracle {}

/** The default Oracle that uses the real world. */
export class WorldOracle extends Oracle {}

export const WORLD_ORACLE = new WorldOracle();
