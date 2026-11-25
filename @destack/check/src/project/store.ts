import { join } from "node:path";
import type { Bucket, BucketEntry } from "@destack/bucket";
import type { Environment } from "@destack/environment";
import { schema, Uuid } from "@destack/schema";

/** The directory below a package keeping its passed test files, one record per inputs digest. */
const LOCAL_DIRECTORY = "node_modules/.cache/destack/test";

/** The variable naming the shared bucket's S3 URL: R2's endpoint, then the bucket's path. */
const URL_VARIABLE = "DESTACK_CI_STORE";

/** The variable holding the bucket's S3 access key identifier, the R2 token's identifier. */
const KEY_VARIABLE = "DESTACK_CI_STORE_KEY_ID";

/** The variable holding the R2 token, whose SHA-256 is the S3 secret key. */
const TOKEN_VARIABLE = "DESTACK_CI_STORE_TOKEN";

/** The region R2 signs every request for. */
const R2_REGION = "auto";

/** The most records read at once: at 40 ms each, 32 read a package's records within 100 ms. */
const CONCURRENCY = 32;

/** The most records durations are read from: about four runs of a package split 16 ways. */
const MOST_RECORDS = 64;

/** One run's test files that passed over its inputs, with how long each took in milliseconds. */
const TestRecord = schema.object({
    files: schema.record(schema.string(), schema.number().nonnegative()),
});

/** One run's test files that passed over its inputs, with how long each took in milliseconds. */
type TestRecord = schema.Infer<typeof TestRecord>;

/** One run of a package's check, with how long it took in milliseconds. */
const CheckRecord = schema.object({ milliseconds: schema.number().nonnegative() });

/** The test files of one package's inputs that passed, as this computer keeps them. */
const LocalRecord = schema.object({ files: schema.array(schema.string()) });

/** The effects the store reads and writes with. */
export type TestStoreEnvironment = Environment<"files" | "process">;

/**
 * The test files that passed over each package's inputs, and how long each took.
 *
 * Records live in the package's directory, and in the shared bucket when the process names one.
 * An inputs digest covers everything a test run reads, so its records hold for any checkout.
 */
export class TestStore {
    /** The shared bucket, absent when the process names none. */
    readonly bucket: Bucket | undefined;
    /** The files the store reads and writes. */
    readonly environment: TestStoreEnvironment;

    /** Keep records on this computer and in a shared bucket. */
    constructor(bucket: Bucket | undefined, environment: TestStoreEnvironment) {
        this.bucket = bucket;
        this.environment = environment;
    }

    /** Open the store the process names, with the shared bucket when its variables are set. */
    static async open(environment: TestStoreEnvironment): Promise<TestStore> {
        // read the bucket's address and credentials
        const variables = environment.process.variables;
        const url = variables[URL_VARIABLE] ?? "";
        const key = variables[KEY_VARIABLE] ?? "";
        const token = variables[TOKEN_VARIABLE] ?? "";

        // keep records on this computer alone
        if (url === "" && key === "" && token === "") {
            return new TestStore(undefined, environment);
        }
        // refuse a store named in part
        else if (url === "" || key === "" || token === "") {
            throw new Error(
                `the test store needs ${URL_VARIABLE}, ${KEY_VARIABLE} and ${TOKEN_VARIABLE} together`,
            );
        }
        // reach the bucket over S3, signing with the token's identifier and its SHA-256, as R2 derives them
        else {
            const { S3Client } = await import("@destack/bucket/s3");
            const address = new URL(url);
            const digest = await environment.crypto.digest(
                "SHA-256",
                new TextEncoder().encode(token),
            );
            const bucket = S3Client.of(
                {
                    location: {
                        endpoint: new URL("/", address),
                        bucket: address.pathname.replace(/^\/|\/$/gu, ""),
                        region: R2_REGION,
                    },
                    credentials: {
                        accessKeyId: key,
                        secretAccessKey: new Uint8Array(digest).toHex(),
                    },
                },
                environment,
            );

            return new TestStore(bucket, environment);
        }
    }

    /** List a package's test files that passed over the same inputs, here or in the bucket. */
    async passed(directory: string, name: string, inputs: string): Promise<Set<string>> {
        // read this computer's record
        const passed = new Set(await this.#local(directory, inputs));

        // add the files of the bucket's records below the inputs digest
        const bucket = this.bucket;
        if (bucket !== undefined) {
            const entries = await TestStore.#entries(bucket, `tests/${name}/${inputs}/`);
            for (const record of await TestStore.#records(bucket, entries)) {
                for (const file of Object.keys(record.files)) {
                    passed.add(file);
                }
            }
        }

        return passed;
    }

    /** Record test files that passed over inputs and their durations, here and in the bucket. */
    async pass(
        directory: string,
        name: string,
        inputs: string,
        durations: ReadonlyMap<string, number>,
    ): Promise<void> {
        // merge the passes into this computer's record
        const { files } = this.environment;
        const kept = await this.#local(directory, inputs);
        const passed = [...new Set([...kept, ...durations.keys()])].toSorted();
        await files.mkdir(join(directory, LOCAL_DIRECTORY), { recursive: true });
        await files.writeTextFile(
            join(directory, LOCAL_DIRECTORY, `${inputs}.json`),
            JSON.stringify({ files: passed }),
        );

        // write the run's passes as one record below the inputs digest
        const bucket = this.bucket;
        if (bucket !== undefined && durations.size > 0) {
            const rounded = new Map(
                [...durations].map(([file, value]) => [file, Math.round(value)]),
            );
            const record = JSON.stringify({ files: Object.fromEntries(rounded) });
            const key = `tests/${name}/${inputs}/${this.#id()}.json`;
            await this.#put(bucket, key, record);
        }
    }

    /** Record how long a package's check took, in the bucket. */
    async check(name: string, milliseconds: number): Promise<void> {
        // write the duration as one record below the package, when the process names a bucket
        const record = JSON.stringify({ milliseconds: Math.round(milliseconds) });
        const key = `checks/${name}/${this.#id()}.json`;
        const bucket = this.bucket;
        if (bucket !== undefined) {
            await this.#put(bucket, key, record);
        }
    }

    /** Write a record to the bucket, reporting a refusal, since a record only saves later work. */
    async #put(bucket: Bucket, key: string, record: string): Promise<void> {
        // report a refusal on standard error
        try {
            await bucket.put(key, record);
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            this.environment.process.stderr.writeSync(`cannot record '${key}': ${message}\n`);
        }
    }

    /** Read how long a package's test files last took, from its newest records over any inputs. */
    async durations(name: string): Promise<Map<string, number>> {
        // read nothing without a bucket
        const durations = new Map<string, number>();
        const bucket = this.bucket;
        if (bucket === undefined) {
            return durations;
        }

        // take each file's duration from the newest record that has it
        const entries = await TestStore.#entries(bucket, `tests/${name}/`);
        const newest = entries
            .toSorted((left, right) => right.uploaded.getTime() - left.uploaded.getTime())
            .slice(0, MOST_RECORDS);
        for (const record of await TestStore.#records(bucket, newest)) {
            for (const [file, milliseconds] of Object.entries(record.files)) {
                if (!durations.has(file)) {
                    durations.set(file, milliseconds);
                }
            }
        }

        return durations;
    }

    /** Read how long a package's check last took, absent when it never ran with the bucket. */
    async checked(name: string): Promise<number | undefined> {
        // read nothing without a bucket
        const bucket = this.bucket;
        if (bucket === undefined) {
            return undefined;
        }

        // read the newest check record
        const entries = await TestStore.#entries(bucket, `checks/${name}/`);
        const newest = entries.toSorted(
            (left, right) => right.uploaded.getTime() - left.uploaded.getTime(),
        )[0];
        const file = newest === undefined ? null : await bucket.get(newest.key);

        return file === null ? undefined : CheckRecord.parse(await file.json()).milliseconds;
    }

    /** Make a record's identifier, a UUIDv7, so its keys sort by time. */
    #id(): string {
        return Uuid.v7(this.environment);
    }

    /** Read this computer's record of an inputs digest's passed files. */
    async #local(directory: string, inputs: string): Promise<readonly string[]> {
        const { files } = this.environment;
        const path = join(directory, LOCAL_DIRECTORY, `${inputs}.json`);

        return (await files.exists(path))
            ? LocalRecord.parse(JSON.parse(await files.readTextFile(path))).files
            : [];
    }

    /** List every file below a prefix, page by page. */
    static async #entries(bucket: Bucket, prefix: string): Promise<BucketEntry[]> {
        // follow the listing's pages
        const entries: BucketEntry[] = [];
        let cursor: string | undefined;
        do {
            const page = await bucket.list({ prefix, ...(cursor === undefined ? {} : { cursor }) });
            entries.push(...page.files);
            cursor = page.cursor;
        } while (cursor !== undefined);

        return entries;
    }

    /** Read test records, at most CONCURRENCY at once. */
    static async #records(bucket: Bucket, entries: readonly BucketEntry[]): Promise<TestRecord[]> {
        // take the next entry until none remain, in each of the concurrent loops
        const queue = entries.toReversed();
        const records: TestRecord[] = [];
        const work = async () => {
            for (let entry = queue.pop(); entry !== undefined; entry = queue.pop()) {
                const file = await bucket.get(entry.key);
                if (file !== null) {
                    records.push(TestRecord.parse(await file.json()));
                }
            }
        };
        await Promise.all(Array.from({ length: Math.min(CONCURRENCY, entries.length) }, work));

        return records;
    }
}
