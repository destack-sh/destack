import { dirname, resolve } from "node:path";
import type { Environment } from "@destack/environment";
import { KeyPair, PublicKey } from "@destack/identity";
import {
    type AwsCredentials,
    Envelope,
    EnvelopeConfiguration,
    UNIVERSE_OWNER,
} from "@destack/identity/aws";
import { schema } from "@destack/schema";
import { ROOT } from "./root.ts";

/** The directory keeping each stage's custody record, beside the release's sources. */
const CUSTODY_DIRECTORY = resolve(ROOT, "custody");

/** The universe's sealed keys: its envelope and its root keyset. */
const UniverseKeys = schema.object({
    /** How the keys are sealed. */
    envelope: EnvelopeConfiguration,
    /** The root keyset, sealed as `envelope` says. */
    rootKey: schema.string().min(1),
});

/** A cloud machine's sealed keys: its envelope, its root keyset, and its key and public half. */
const MachineKeys = UniverseKeys.extend({
    /** The public half of the machine's key, as the universe declares it. */
    publicKey: PublicKey,
    /** The machine's private key, as `KeyPair.generate` writes it, sealed as `envelope` says. */
    key: schema.string().min(1),
});
/** A cloud machine's sealed keys. */
export type MachineKeys = schema.Infer<typeof MachineKeys>;

/** A stage's custody record: the universe's sealed keys and each cloud machine's, by its id. */
const CustodyRecord = schema.object({
    /** The universe's keys. */
    universe: UniverseKeys,
    /** Each cloud machine's keys, by machine id. */
    machines: schema.record(schema.string(), MachineKeys),
});
/** A stage's custody record. */
type CustodyRecord = schema.Infer<typeof CustodyRecord>;

/** The KMS keys a stage envelopes under: the universe's and each cloud machine's, by machine id. */
export interface CustodyEnvelopes {
    /** The universe's envelope. */
    readonly universe: EnvelopeConfiguration;
    /** Each cloud machine's envelope, by machine id. */
    readonly machines: ReadonlyMap<string, EnvelopeConfiguration>;
}

/** A stage's key custody: each Worker's sealed root keys and each cloud machine's sealed key. */
export class Custody {
    /** The record. */
    readonly record: CustodyRecord;

    /** Keep a stage's record. */
    constructor(record: CustodyRecord) {
        this.record = record;
    }

    /** Generate keys in the clear for local runs and rehearsals, which deploy nothing. */
    static async local(
        machines: readonly string[],
        environment: Pick<Environment, "clock" | "crypto" | "network" | "random">,
    ): Promise<Custody> {
        // keep plain root keysets and plain machine keys under the local envelope
        const local = { kind: "local" } as const;
        const entries = await Promise.all(
            machines.map(
                async (id) =>
                    [id, await Custody.#machineKeys(id, local, undefined, environment)] as const,
            ),
        );
        const universe = new Envelope(local, UNIVERSE_OWNER, undefined, environment);

        return new Custody({
            universe: { envelope: local, rootKey: await universe.generate(environment) },
            machines: Object.fromEntries(entries),
        });
    }

    /** Write the file of a stage's record. */
    static file(stage: string): string {
        return resolve(CUSTODY_DIRECTORY, `${stage}.json`);
    }

    /** Read a stage's record as a deploy does, refusing a missing key or one sealed otherwise. */
    static async read(
        file: string,
        envelopes: CustodyEnvelopes,
        environment: Environment<"files">,
    ): Promise<Custody> {
        // refuse a stage the operator has not sealed
        if (!(await environment.files.exists(file))) {
            throw new Error(
                `the custody record '${file}' is missing: the operator seals it with custody seal and commits it`,
            );
        }

        // refuse a key the record lacks, or keeps under another envelope than its root's
        const record = CustodyRecord.parse(JSON.parse(await environment.files.readTextFile(file)));
        Custody.#require("the universe", record.universe, envelopes.universe);
        for (const [id, envelope] of envelopes.machines) {
            Custody.#require(`machine ${id}`, record.machines[id], envelope);
        }

        return new Custody(record);
    }

    /** Seal the keys a stage's record lacks as the operator does, refusing one sealed otherwise. */
    static async seal(
        file: string,
        envelopes: CustodyEnvelopes,
        credentials: AwsCredentials,
        environment: Environment<"files">,
    ): Promise<Custody> {
        // read the stored record, if any
        const stored = (await environment.files.exists(file))
            ? CustodyRecord.parse(JSON.parse(await environment.files.readTextFile(file)))
            : undefined;

        // keep the universe's keys, else generate a root keyset under its envelope
        const universe =
            stored === undefined
                ? {
                      envelope: envelopes.universe,
                      rootKey: await new Envelope(
                          envelopes.universe,
                          UNIVERSE_OWNER,
                          credentials,
                          environment,
                      ).generate(environment),
                  }
                : Custody.#require("the universe", stored.universe, envelopes.universe);

        // keep each machine's keys, the undeclared ones' among them, else generate a root keyset and key
        const machines: Record<string, MachineKeys> = { ...stored?.machines };
        for (const [id, envelope] of envelopes.machines) {
            const kept = stored?.machines[id];
            machines[id] =
                kept === undefined
                    ? await Custody.#machineKeys(id, envelope, credentials, environment)
                    : Custody.#require(`machine ${id}`, kept, envelope);
        }

        // write the record when it changed
        const record = CustodyRecord.parse({ universe, machines });
        const text = `${JSON.stringify(record, undefined, 4)}\n`;
        if (stored === undefined || JSON.stringify(stored) !== JSON.stringify(record)) {
            await environment.files.mkdir(dirname(file), { recursive: true });
            await environment.files.writeTextFile(file, text);
        }

        return new Custody(record);
    }

    /** Read a cloud machine's keys, refusing a machine the record lacks. */
    machine(id: string): MachineKeys {
        const keys = this.record.machines[id];
        if (keys === undefined) {
            throw new Error(`the custody record keeps no keys of machine '${id}'`);
        }

        return keys;
    }

    /** Require kept keys sealed under their root's envelope, never regenerated under another. */
    static #require<Keys extends { readonly envelope: EnvelopeConfiguration }>(
        owner: string,
        keys: Keys | undefined,
        envelope: EnvelopeConfiguration,
    ): Keys {
        // refuse missing keys, which only the operator seals
        if (keys === undefined) {
            throw new Error(
                `the custody record keeps no keys of '${owner}': the operator seals them`,
            );
        }
        // refuse keys under another envelope, as regenerating them would orphan what they sealed
        // TODO #Incomplete: rewrap moved keys through KMS ReEncrypt as an operator verb, as Vault migrates seals
        else if (!isSame(keys.envelope, envelope)) {
            throw new Error(
                `the keys of '${owner}' are sealed under ${JSON.stringify(keys.envelope)}, not ${JSON.stringify(envelope)}: rewrap them, never regenerate`,
            );
        }

        return keys;
    }

    /** Generate a cloud machine's root keyset and key under its envelope, the key sealed for it. */
    static async #machineKeys(
        id: string,
        configuration: EnvelopeConfiguration,
        credentials: AwsCredentials | undefined,
        environment: Pick<Environment, "clock" | "crypto" | "network" | "random">,
    ): Promise<MachineKeys> {
        const envelope = new Envelope(
            configuration,
            Envelope.machine(id),
            credentials,
            environment,
        );
        const { pair, secret } = await KeyPair.generate(environment.crypto);

        return {
            envelope: configuration,
            rootKey: await envelope.generate(environment),
            publicKey: pair.publicKey,
            key: await envelope.seal(secret),
        };
    }
}

/** Compare two envelopes by their configuration. */
function isSame(left: EnvelopeConfiguration, right: EnvelopeConfiguration): boolean {
    return JSON.stringify(left) === JSON.stringify(right);
}
