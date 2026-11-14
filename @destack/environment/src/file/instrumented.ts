import { ENVIRONMENT_SCOPE, SpanRunner } from "../telemetry/runner.ts";
import { SpanPath } from "../telemetry/span.ts";
import type { Telemetry } from "../telemetry/telemetry.ts";
import type { FileSystem } from "./file.ts";

/** The running spans' total of quick file reads. */
const READS = "destack.file.read.count";

/** The running spans' total of quick file writes, copies, renames and removals. */
const WRITES = "destack.file.write.count";

/** A file system spanning its slow verbs and counting its quick ones in the running spans. */
export class InstrumentedFileSystem implements FileSystem {
    /** The file system the verbs go to. */
    readonly source: FileSystem;
    /** The runner of the slow verbs' spans. */
    readonly #span: SpanRunner;
    /** The telemetry whose running spans count the quick verbs. */
    readonly #telemetry: Telemetry;

    /** Instrument a file system on a telemetry. */
    constructor(source: FileSystem, telemetry: Telemetry) {
        // take the source, the spans' runner and the telemetry counting quick verbs
        this.source = source;
        this.#telemetry = telemetry;
        this.#span = SpanRunner.of(
            telemetry.tracers.getTracer(ENVIRONMENT_SCOPE),
            telemetry.meters.getMeter(ENVIRONMENT_SCOPE),
            telemetry,
        );
    }

    /** Read the uninstrumented file system below one, itself when uninstrumented. */
    static sourceOf(files: FileSystem): FileSystem {
        return files instanceof InstrumentedFileSystem ? files.source : files;
    }

    /** Run a slow verb on a path in its span. */
    #run<Value>(name: string, path: string, run: () => Promise<Value>): Promise<Value> {
        return this.#span(name, { "file.path": path }, run);
    }

    /** Count a quick verb in the running spans' totals. */
    #count<Value>(total: string, run: () => Promise<Value>): Promise<Value> {
        SpanPath.add(this.#telemetry.context.active(), total, 1);

        return run();
    }

    /** Read a file's bytes. */
    readonly readFile: FileSystem["readFile"] = (path) =>
        this.#count(READS, () => this.source.readFile(path));

    /** Read a file's text. */
    readonly readTextFile: FileSystem["readTextFile"] = (path) =>
        this.#count(READS, () => this.source.readTextFile(path));

    /** Write a file's bytes. */
    readonly writeFile: FileSystem["writeFile"] = (path, data, options) =>
        this.#count(WRITES, () => this.source.writeFile(path, data, options));

    /** Write a file's text. */
    readonly writeTextFile: FileSystem["writeTextFile"] = (path, text, options) =>
        this.#count(WRITES, () => this.source.writeTextFile(path, text, options));

    /** List a directory's entries. */
    readonly readDirectory: FileSystem["readDirectory"] = (path, options) =>
        this.#run("destack.file.list", path, () => this.source.readDirectory(path, options));

    /** Create a directory. */
    readonly mkdir: FileSystem["mkdir"] = (path, options) =>
        this.#count(WRITES, () => this.source.mkdir(path, options));

    /** Create a fresh temporary directory. */
    readonly makeTemporaryDirectory: FileSystem["makeTemporaryDirectory"] = (prefix, options) =>
        this.#count(WRITES, () => this.source.makeTemporaryDirectory(prefix, options));

    /** Remove a path. */
    readonly remove: FileSystem["remove"] = (path, options) =>
        options?.recursive === true
            ? this.#run("destack.file.remove", path, () => this.source.remove(path, options))
            : this.#count(WRITES, () => this.source.remove(path, options));

    /** Move a path. */
    readonly rename: FileSystem["rename"] = (from, to) =>
        this.#count(WRITES, () => this.source.rename(from, to));

    /** Copy a file. */
    readonly copyFile: FileSystem["copyFile"] = (from, to) =>
        this.#count(WRITES, () => this.source.copyFile(from, to));

    /** Copy a file or a tree. */
    readonly copy: FileSystem["copy"] = (from, to) =>
        this.#run("destack.file.copy", from, () => this.source.copy(from, to));

    /** Wait for a file's lock. */
    readonly lock: FileSystem["lock"] = (path, mode, options) =>
        this.#run("destack.file.lock", path, () => this.source.lock(path, mode, options));

    /** Report whether a path exists. */
    readonly exists: FileSystem["exists"] = (path) => this.source.exists(path);

    /** Read a path's status. */
    readonly stat: FileSystem["stat"] = (path) => this.source.stat(path);

    /** Read a path's own status, not following a link. */
    readonly lstat: FileSystem["lstat"] = (path) => this.source.lstat(path);

    /** Resolve a path through its links. */
    readonly realPath: FileSystem["realPath"] = (path) => this.source.realPath(path);

    /** Join path names. */
    readonly join: FileSystem["join"] = (directory, ...names) =>
        this.source.join(directory, ...names);

    /** Link a path to a target. */
    readonly symlink: FileSystem["symlink"] = (target, path) => this.source.symlink(target, path);

    /** Read a link's target. */
    readonly readLink: FileSystem["readLink"] = (path) => this.source.readLink(path);

    /** Hard-link a path to a target. */
    readonly link: FileSystem["link"] = (target, path) => this.source.link(target, path);

    /** Change a path's mode. */
    readonly chmod: FileSystem["chmod"] = (path, mode) => this.source.chmod(path, mode);

    /** Open a file. */
    readonly open: FileSystem["open"] = (path, options) => this.source.open(path, options);

    /** Watch a path's changes. */
    readonly watch: FileSystem["watch"] = (path, options) => this.source.watch(path, options);

    /** Find the person's runtime directory. */
    readonly runtimeDirectory: FileSystem["runtimeDirectory"] = () =>
        this.source.runtimeDirectory();

    /** Find the person's cache directory. */
    readonly cacheDirectory: FileSystem["cacheDirectory"] = () => this.source.cacheDirectory();

    /** Lock a file now, absent while another holder has it. */
    readonly tryLock: FileSystem["tryLock"] = (path, mode) => this.source.tryLock(path, mode);
}
