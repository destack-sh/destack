import hljs from "highlight.js/lib/core";
import javascript from "highlight.js/lib/languages/javascript";
import x86asm from "highlight.js/lib/languages/x86asm";

import type { snippets } from "./generated/snippets";

hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("x86asm", x86asm);

export type SnippetKey = keyof typeof snippets;

export type OutputTarget = "assembly" | "javascript" | "output";

export type CompiledExample = Partial<Record<OutputTarget, string>>;

export type ExampleTopic = {
    isDiagnostic?: boolean;
    key: SnippetKey;
    label: string;
};

export type ExampleCategory = {
    label: string;
    topics: readonly ExampleTopic[];
};

export type ExampleArea = {
    label: string;
    categories: readonly ExampleCategory[];
};

export const exampleAreas = [
    {
        label: "Language",
        categories: [
            category("Types", [
                topic("types/primitives", "primitives"),
                diagnosticTopic("types/intervals", "intervals"),
                diagnosticTopic("types/newtypes", "newtypes"),
                topic("types/interfaces", "interfaces"),
                diagnosticTopic("types/any", "any"),
                topic("types/extensions", "extensions"),
                topic("types/enums", "enums"),
                topic("types/structs", "structs"),
                topic("types/sequences", "sequences"),
                diagnosticTopic("types/readonly", "readonly"),
                topic("types/generics", "generics"),
                topic("types/constraints", "constraints"),
                topic("types/associated", "associated"),
                topic("types/reflection", "reflection"),
                topic("types/tagged-unions", "tagged"),
                topic("types/static", "static"),
            ]),
            category("Expressions", [
                topic("expressions/blocks", "blocks"),
                topic("expressions/captures", "captures"),
                topic("expressions/continuations", "continuations"),
                topic("expressions/patterns", "patterns"),
                topic("expressions/is", "is"),
                topic("expressions/loops", "loops"),
                topic("expressions/using", "using"),
                topic("expressions/ranges", "ranges"),
                topic("expressions/overloads", "overloads"),
                diagnosticTopic("expressions/errors", "errors"),
                topic("expressions/tsx", "tsx"),
                topic("expressions/decorators", "decorators"),
                topic("expressions/module", "module"),
                topic("expressions/comptime", "comptime"),
                topic("expressions/macros", "macros"),
            ]),
            category("Memory", [
                topic("memory/space", "space"),
                topic("memory/ownership", "ownership"),
                diagnosticTopic("memory/borrowing", "borrowing"),
                topic("memory/lifetimes", "lifetimes"),
                topic("memory/drop", "drop"),
                topic("memory/conversions", "conversions"),
                topic("memory/unsafe", "unsafe"),
                topic("memory/algebra", "algebra"),
                topic("memory/polymorphism", "polymorphism"),
                topic("memory/capabilities", "capabilities"),
                topic("memory/sync", "sync"),
            ]),
        ],
    },
    {
        label: "Runtime",
        categories: [
            category("Modules", [
                topic("runtime/modules", "modules"),
                topic("runtime/conditions", "conditions"),
                topic("runtime/import-meta", "import-meta"),
                topic("runtime/data-modules", "data"),
                topic("runtime/policy", "policy"),
            ]),
            category("Tests", [
                topic("runtime/test", "test"),
                topic("runtime/fuzz", "fuzz"),
                topic("runtime/bench", "bench"),
                topic("runtime/simulation", "simulation"),
            ]),
        ],
    },
] as const satisfies readonly ExampleArea[];

export const compiledExamples = {
    "types/primitives": compiled(
        `loadUser:
    call    readUserRow
    call    decodeUserRow
    test    r2b, 1
    setnz   r3b
    mov     r0, [r2 + UserRow.id]
    mov     r1, [r2 + UserRow.name]
    ret`,
        `export function loadUser() {
  const raw = readUserRow();
  const row = decodeUserRow(raw).try();

  return Result.ok({
    id: UserId(row.id),
    name: row.name,
    isAdmin: (row.flags & 1) === 1,
  });
}`,
    ),
    "types/intervals": diagnostic(
        `error destack1201: numeric literal does not satisfy UserPort
  --> types/intervals.ds:15:27
   |
15 | const invalid: UserPort = 70000;
   |                           ^^^^^ expected 1024..=65535`,
    ),
    "types/any": diagnostic(
        `error destack1003: native .ds modules do not support 'any'
  --> types/any.ds:3:13
   |
 3 | let value: any = readInput();
   |            ^^^ use 'unknown' and narrow before use`,
    ),
    "types/newtypes": diagnostic(
        `error destack1302: backing value is not assignable to newtype
  --> types/newtypes.ds:9:25
   |
 9 | const invalid: UserId = 42;
   |                         ^^ construct explicitly with UserId(42)`,
    ),
    "types/readonly": diagnostic(
        `error destack2104: cannot assign through deep readonly view
  --> types/readonly.ds:15:1
   |
15 | user.profile.name = "Grace";
   | ^^^^^^^^^^^^^^^^^ mutation requires User, not readonly User`,
    ),
    "types/generics": compiled(
        `first<Page<User,64>>:
    lea     rax, [rdi + Page.items]
    ret

sortUsers:
    mov     rdi, rdi
    call    slice.sort
    ret`,
        `export function first(page) {
  return page.items[0];
}`,
    ),
    "types/tagged-unions": compiled(
        `estimatedBytes:
    mov     eax, [rdi + tag]
    cmp     eax, 0
    je      .inline
    cmp     eax, 1
    je      .blob
    xor     eax, eax
    ret
.inline:
    mov     rax, [rdi + bytes.length]
    ret
.blob:
    mov     rax, [rdi + blob.bytes]
    ret`,
        `export function estimatedBytes(command) {
  switch (command.kind) {
    case "putInline": return BigInt(command.bytes.length);
    case "putBlob": return command.blob.bytes;
    case "delete": return 0n;
  }
}`,
    ),
    "types/static": compiled(
        `sizeOfDocumentPage:
    mov     eax, 4096
    ret

alignOfDocumentPage:
    mov     eax, 1
    ret`,
        `export const DocumentPage = {
  size: 4096,
  align: 1,
};`,
    ),
    "expressions/blocks": compiled(
        `loadProfile:
    call    loadUserRecord
    test    [rax + displayName.tag], 1
    jz      .email
    mov     r1, [rax + displayName.value]
    jmp     .done
.email:
    mov     r1, [rax + email]
.done:
    mov     r0, [rax + id]
    ret`,
        `export function loadProfile(id) {
  const record = loadUserRecord(id).try();
  const label = record.displayName ?? record.email;

  return Result.ok({ id: record.id, label });
}`,
    ),
    "expressions/captures": compiled(
        `next:
    mov     rax, [closure.count]
    inc     rax
    mov     [closure.count], rax
    ret

snapshot:
    mov     rax, [capture.count]
    ret`,
        `let count = 0;

export const next = () => {
  count += 1;
  return count;
};`,
    ),
    "expressions/patterns": compiled(
        `route:
    cmp     [rdi + tag], Message.write
    jne     .delete
    mov     rax, [rdi + bytes.length]
    cmp     rax, 4096
    ja      .blob
    call    Route.Inline
    ret
.blob:
    call    hash
    call    Route.Blob
    ret
.delete:
    call    Route.Tombstone
    ret`,
        `export function route(message) {
  if (message.kind === "write" && message.bytes.length <= 4096) {
    return Route.Inline({ id: message.id, bytes: message.bytes });
  }

  if (message.kind === "write") {
    return Route.Blob({ id: message.id, hash: hash(message.bytes) });
  }

  return Route.Tombstone({ id: message.id });
}`,
    ),
    "expressions/overloads": compiled(
        `Bytes.add:
    mov     rax, [rdi + Bytes.value]
    add     rax, [rsi + Bytes.value]
    mov     [rsp + Bytes.value], rax
    lea     rax, [rsp]
    ret

total:
    call    Bytes.add
    ret`,
        `export function total(left, right) {
  return Bytes.add(left, right);
}`,
    ),
    "expressions/is": compiled(
        `label:
    cmp     [rdi + tag], User
    jne     .string
    mov     rax, [rdi + User.name]
    ret
.string:
    mov     rax, rdi
    ret`,
        `export function label(value) {
  return value.kind === "user" ? value.name : value;
}`,
    ),
    "expressions/errors": diagnostic(
        `error destack3107: Result must be opened before use
  --> expressions/errors.ds:8:18
   |
 8 |     const text = readConfig(path);
   |                  ^^^^^^^^^^^^^^^^ use '?' or return the Result`,
    ),
    "expressions/tsx": compiled(
        `view:
    call    Status
    mov     [rsp + 0], rax
    call    Log
    mov     [rsp + 8], rax
    lea     rdi, [rsp]
    call    Panel
    ret`,
        `export const view = Panel({
  title: "build",
  children: [
    Status({ state: status }),
    Log({ lines: events }),
  ],
});`,
    ),
    "expressions/comptime": compiled(
        `mask$1024:
    mov     eax, 1023
    ret

IndexTable.layout:
    .size   4096
    .align  4`,
        `export const mask1024 = 1023;

export const IndexTable = {
  buckets: 1024,
  bytes: 4096,
};`,
    ),
    "memory/space": compiled(
        `recordRead:
    mov     rax, [rdi + RequestStats.bytes]
    lock    add [stats.reads], rax
    ret

stats:
    .quad   0
    .quad   0`,
        `export function recordRead(request) {
  stats.reads.add(request.bytes);
}`,
    ),
    "memory/borrowing": diagnostic(
        `error destack4202: exclusive borrow overlaps live borrow
  --> memory/borrowing.ds:13:18
   |
13 | let exclusiveX = &exclusive point.x;
   |                  ^^^^^^^^^^^^^^^^^^ readX is still live`,
    ),
    "memory/lifetimes": compiled(
        `selectedName:
    test    dl, dl
    cmovnz  rax, rdi
    cmovz   rax, rsi
    lea     rax, [rax + Node.name]
    ret`,
        `export function selectedName(left, right, active) {
  const selected = active ? left : right;

  return selected.name;
}`,
    ),
    "memory/drop": compiled(
        `copyWithDispose:
    call    openFile
    mov     r12, rax
    call    openFile
    mov     r13, rax
    call    copy
    mov     rdi, r13
    call    FileHandle.dispose
    mov     rdi, r12
    call    FileHandle.dispose
    ret`,
        `export function copyWithDispose(inputPath, outputPath) {
  using input = openFile(inputPath).try();
  using output = openFile(outputPath).try();

  return copy(input, output);
}`,
    ),
    "memory/algebra": compiled(
        `inspect:
    mov     rax, rdi
    ret

ReadBuffer:
    .type   ref<Buffer, borrowed, readonly>`,
        `export function inspect(buffer) {
  return buffer;
}`,
    ),
    "memory/sync": compiled(
        `submit:
    mov     rdi, [rdi + queue.ptr]
    mov     rsi, rsi
    call    ConcurrentQueue.push
    xor     eax, eax
    ret`,
        `export function submit(queue, job) {
  queue.push(job);

  return Result.ok(undefined);
}`,
    ),
    "runtime/modules": compiled(
        `loadUser:
    mov     rax, [rel database]
    mov     rdi, rax
    call    Database.loadUser
    ret

.module.role:
    .ascii  "server"`,
        `export function loadUser(id) {
  return database.loadUser(id);
}`,
    ),
    "runtime/data-modules": compiled(
        `manifest:
    .incbin "./destack.json"

schemaText:
    .incbin "./schema.sql"

wasmBytes:
    .incbin "./parser.wasm"

packageName:
    .quad   manifest.name`,
        `import manifest from "./destack.json" with { type: "json" };
import schemaText from "./schema.sql" with { type: "text" };
import wasmBytes from "./parser.wasm" with { type: "bytes" };

export const packageName = manifest.name;`,
    ),
    "runtime/test": compiled(
        `test_parsePort_accepts_user_ports:
    lea     rdi, [rel string_8080]
    call    parsePort
    cmp     eax, 8080
    jne     .fail
    xor     eax, eax
    ret
.fail:
    mov     eax, 1
    ret`,
        `test("parsePort accepts user ports", () => {
  const port = parsePort("8080").try();

  expect(port).toBe(8080);
});`,
    ),
    "runtime/fuzz": compiled(
        `fuzz_document_id_roundtrip:
    call    Generator.uint64
.loop:
    call    encode
    call    decodeDocumentId
    cmp     rax, rbx
    jne     .shrink
    dec     rcx
    jnz     .loop
    ret
.shrink:
    call    corpus.shrink`,
        `fuzz("document id roundtrip", documentIds, { samples: 10000 }, (id) => {
  const decoded = decodeDocumentId(encode(id)).try();

  expect(decoded).toBe(id);
});`,
    ),
    "runtime/bench": compiled(
        `bench_decode_index_page:
    call    bench.start
.loop:
    call    decodeIndex
    add     r12, 4096
    call    bench.more
    test    al, al
    jnz     .loop
    mov     rax, r12
    ret`,
        `bench("decode index page", options, (ctx) => {
  const page = loadFixture("index.page");

  const result = ctx.iter(() => decodeIndex(page));
  ctx.bytes(4096 * result.iterations);
});`,
    ),
    "runtime/simulation": compiled(
        `sim_replica_survives_stale_reads:
    call    World.injectFault
    call    startCluster
    call    Cluster.write
    call    World.runUntilIdle
    call    Cluster.read
    call    assertEqual
    ret`,
        `simulation("replica survives stale reads", async (ctx) => {
  ctx.fault(staleRead, { probabilityPpm: 250000 });
  const cluster = await startCluster(ctx.world).try();

  await cluster.write(key, document).try();
  expect(await cluster.read(key).try()).toEqual(document);
});`,
    ),
} satisfies Partial<Record<SnippetKey, CompiledExample>>;

export function outputFor(key: SnippetKey, target: OutputTarget): string {
    const example = compiledExample(key);
    return example[target] ?? "";
}

export function highlightedOutputFor(key: SnippetKey, target: OutputTarget): string {
    const output = outputFor(key, target);

    if (target === "assembly") {
        return hljs.highlight(output, { language: "x86asm" }).value;
    }

    if (target === "javascript") {
        return hljs.highlight(output, { language: "javascript" }).value;
    }

    return highlightDiagnostic(output);
}

export function targetsFor(topic: ExampleTopic): readonly OutputTarget[] {
    if (topic.isDiagnostic) {
        return ["output"];
    }

    return ["assembly", "javascript"];
}

function compiledExample(key: SnippetKey): CompiledExample {
    return compiledExamples[key] ?? compiled(
        `${labelFor(key)}:
    ; compiler output pending
    ret`,
        `// generated js for ${key}
export {};`,
    );
}

function category(label: string, topics: readonly ExampleTopic[]): ExampleCategory {
    return { label, topics };
}

function topic(key: SnippetKey, label: string): ExampleTopic {
    return { key, label };
}

function diagnosticTopic(key: SnippetKey, label: string): ExampleTopic {
    return { isDiagnostic: true, key, label };
}

function compiled(assembly: string, javascript: string): CompiledExample {
    return { assembly, javascript };
}

function diagnostic(value: string): CompiledExample {
    return { output: value };
}

function highlightDiagnostic(output: string): string {
    const lines = output.split("\n");
    const body = lines.map(highlightDiagnosticLine).join("\n");

    return `<span data-k="diagnostic-shadow"><span data-k="diagnostic-box">${body}</span></span>`;
}

function highlightDiagnosticLine(line: string): string {
    const header = line.match(/^(error) (destack\d+): (.+)$/);
    if (header != undefined) {
        const [, level, code, message] = header;

        return [
            `<span data-k="diagnostic-header">`,
            `<span data-k="diagnostic-level">${escapeHtml(level)}</span> `,
            `<span data-k="diagnostic-code">${escapeHtml(code)}</span>`,
            `<span data-k="punct">:</span> `,
            `${escapeHtml(message)}`,
            `</span>`,
        ].join("");
    }

    const location = line.match(/^(\s*-->\s*)(.+)$/);
    if (location != undefined) {
        const [, prefix, path] = location;

        return [
            `<span data-k="diagnostic-location">`,
            `${escapeHtml(prefix)}`,
            `<span data-k="diagnostic-path">${escapeHtml(path)}</span>`,
            `</span>`,
        ].join("");
    }

    const source = line.match(/^(\s*\d+)( \| )(.*)$/);
    if (source != undefined) {
        const [, number, pipe, code] = source;

        return [
            `<span data-k="diagnostic-source">`,
            `<span data-k="diagnostic-line">${escapeHtml(number)}</span>`,
            `<span data-k="diagnostic-pipe">${escapeHtml(pipe)}</span>`,
            `${escapeHtml(code)}`,
            `</span>`,
        ].join("");
    }

    const marker = line.match(/^(\s*\|\s*)(\^+)(.*)$/);
    if (marker != undefined) {
        const [, prefix, carets, note] = marker;

        return [
            `<span data-k="diagnostic-marker">`,
            `<span data-k="diagnostic-pipe">${escapeHtml(prefix)}</span>`,
            `<span data-k="diagnostic-caret">${escapeHtml(carets)}</span>`,
            `<span data-k="diagnostic-note">${escapeHtml(note)}</span>`,
            `</span>`,
        ].join("");
    }

    if (/^\s*\|/.test(line)) {
        return `<span data-k="diagnostic-pipe">${escapeHtml(line)}</span>`;
    }

    return escapeHtml(line);
}

function escapeHtml(value: string): string {
    return value
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll("\"", "&quot;");
}

function labelFor(key: SnippetKey): string {
    return key.replaceAll("/", "_").replaceAll("-", "_");
}
