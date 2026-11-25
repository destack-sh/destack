import type { Grammar } from "@destack/code";
import type { Block, TextSpan } from "@destack/object";
import { t } from "@destack/locale";
import { type JsonValue, schema } from "@destack/schema";
import * as style from "@destack/style";
import { motion, space } from "@destack/theme/tokens.stylex";
import {
    type Component,
    createComponent,
    createContext,
    createMemo,
    createUniqueId,
    dynamic,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { Checkbox } from "../checkbox/index.ts";
import {
    CodeBlock,
    CodeBlockContent,
    CodeBlockHeader,
    CodeBlockTitle,
} from "../code-block/index.ts";
import { CopyButton } from "../copy-button/index.ts";
import type { ElementPartProperties } from "../part/index.ts";
import { Prose } from "../prose/index.ts";
import "../prose/prose.css";
import { createReveal, type Reveal, type RichTextReveal } from "./reveal.ts";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "../table/index.ts";

/** Text coming up. */
const rise = style.keyframes({
    from: { opacity: 0 },
});

/** The styles of rich text. */
const styles = style.create({
    todos: {
        paddingInlineStart: 0,
        listStyleType: "none",
    },
    todo: {
        display: "flex",
        alignItems: "baseline",
        gap: space[2],
    },
    todoText: {
        flexGrow: 1,
        minWidth: 0,
    },
    fading: {
        animationDuration: motion.durationMedium,
        animationName: rise,
        animationTimingFunction: "ease-out",
    },
    // come up with the first line in it
    marked: {
        opacity: {
            default: 1,
            ":is(:has([data-slot=rich-text-stretch]):not(:has([data-revealed])))": 0,
        },
        transitionDuration: motion.durationLong,
        transitionProperty: "opacity",
        transitionTimingFunction: "ease-out",
    },
    waiting: {
        opacity: { default: 0, ":is([data-revealed])": 1 },
        animationDuration: motion.durationLong,
        animationName: { default: "none", ":is([data-revealed])": rise },
        animationTimingFunction: "ease-out",
    },
    align: (align: string) => ({
        textAlign: align,
    }),
});

/** The addresses links lead to, all others staying text. */
const ADDRESSES = /^(?:https?:|mailto:|tel:|#|\/(?!\/))/iu;

/** The addresses of other sites, which open beside the page. */
const OUTSIDE = /^https?:/iu;

/** The value of a heading. */
const Heading = schema.looseObject({ level: schema.number().int().min(1).max(6) });

/** The value of a list's item. */
const Item = schema.looseObject({ style: schema.string(), start: schema.number().exactOptional() });

/** The value of a todo. */
const Todo = schema.looseObject({ isChecked: schema.boolean() });

/** The value of a block of code. */
const Code = schema.looseObject({ language: schema.string().exactOptional() });

/** The value of an image. */
const Image = schema.looseObject({ source: schema.string(), alternative: schema.string() });

/** The value of a table. */
const Columns = schema.looseObject({ alignments: schema.array(schema.string()) });

/** The value of a table's row. */
const Row = schema.looseObject({ isHeader: schema.boolean() });

/** The value of a link. */
const Link = schema.looseObject({ href: schema.string() });

/** The properties of what draws a block. */
export interface RichTextBlockProperties {
    /** The block. */
    readonly block: Block;
    /** Its text and the blocks in it, drawn. */
    readonly children: JSX.Element;
}

/** The properties of what draws a mark. */
export interface RichTextMarkProperties {
    /** The mark's value, such as a link's address. */
    readonly value: JsonValue;
    /** The marked text, drawn. */
    readonly children: JSX.Element;
}

/** The properties of what draws an inline node. */
export interface RichTextNodeProperties {
    /** The node's value, such as a mention's subject. */
    readonly value: JsonValue;
}

/** What draws the types of a text, the kit's own for those left out. */
export interface RichTextComponents {
    /** What draws each type of block. */
    readonly blocks?: Readonly<Record<string, Component<RichTextBlockProperties>>>;
    /** What draws each type of mark. */
    readonly marks?: Readonly<Record<string, Component<RichTextMarkProperties>>>;
    /** What draws each type of inline node. */
    readonly nodes?: Readonly<Record<string, Component<RichTextNodeProperties>>>;
}

/** The properties of rich text, the native element's attributes included. */
export type RichTextProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "children"> &
    ElementPartProperties & {
        /** The blocks, of a field, of objects or of Markdown. */
        readonly blocks: readonly Block[];
        /** Whether the text is still being written. */
        readonly isStreaming?: boolean;
        /** What text comes up by while it is written, the lines people see by default. */
        readonly reveal?: RichTextReveal;
        /** What draws its types, the kit's own for those left out. */
        readonly components?: RichTextComponents;
        /** Report whether an image's address may load, none by default. */
        readonly allowsImage?: (source: string) => boolean;
        /** Find the grammar that colors a language's code, none by default. */
        readonly grammar?: (language: string) => Grammar | undefined;
        /** Handle a person checking a todo, which none can by default. */
        readonly onTodoChange?: (block: Block, isChecked: boolean) => void;
    };

/** What the blocks of a rich text read from it. */
interface RichTextControl {
    /** What draws its types. */
    readonly components: () => RichTextComponents;
    /** Report whether an image's address may load. */
    readonly allowsImage: (source: string) => boolean;
    /** Find the grammar that colors a language's code. */
    readonly grammar: (language: string) => Grammar | undefined;
    /** Report whether the text is still being written. */
    readonly isStreaming: () => boolean;
    /** How text comes up as it is written. */
    readonly reveal: Pick<Reveal, "isWaiting" | "isFading" | "add">;
    /** Read what handles a person checking a todo. */
    readonly onTodoChange: () => ((block: Block, isChecked: boolean) => void) | undefined;
}

/** The rich text around its blocks. */
const RichTextContext = createContext<RichTextControl>({
    components: () => ({}),
    allowsImage: () => false,
    grammar: () => undefined,
    isStreaming: () => false,
    reveal: { isWaiting: () => false, isFading: () => false, add: () => {} },
    onTodoChange: () => undefined,
});

/** Render rich text: its blocks as they change, each drawn by its type. */
export function RichText(properties: RichTextProperties): JSX.Element {
    // bring the text up as it is written
    const rest = omit(
        properties,
        "blocks",
        "isStreaming",
        "reveal",
        "components",
        "allowsImage",
        "grammar",
        "onTodoChange",
    );
    const reveal = createReveal({
        blocks: () => properties.blocks,
        isStreaming: () => properties.isStreaming === true,
        unit: () => properties.reveal ?? "line",
    });
    const control: RichTextControl = {
        components: () => properties.components ?? {},
        allowsImage: (source) => properties.allowsImage?.(source) ?? false,
        grammar: (language) => properties.grammar?.(language),
        isStreaming: () => properties.isStreaming === true,
        reveal,
        onTodoChange: () => properties.onTodoChange,
    };

    return (
        <RichTextContext value={control}>
            <Prose
                {...rest}
                aria-busy={properties.isStreaming === true ? "true" : undefined}
                data-streaming={properties.isStreaming === true ? "" : undefined}
            >
                <Blocks blocks={reveal.blocks()} />
            </Prose>
        </RichTextContext>
    );
}

/** Blocks that stand alone, or the items of one list. */
interface Group {
    /** What tells it from the others. */
    readonly key: string;
    /** The list's element and look, absent for a block alone. */
    readonly list?: "bullet" | "number" | "todo";
    /** The blocks. */
    readonly blocks: readonly Block[];
}

/** Render blocks, the items of a list in one. */
function Blocks(properties: {
    /** The blocks. */
    readonly blocks: readonly Block[];
}): JSX.Element {
    const groups = createMemo(() => groupsOf(properties.blocks));

    return (
        <For each={groups()} keyed={(group) => group.key}>
            {(group) => (
                <Show
                    when={group().list}
                    fallback={
                        <For each={group().blocks} keyed={keyOf}>
                            {(block) => <BlockView block={block()} />}
                        </For>
                    }
                >
                    {(list) => <List list={list()} blocks={group().blocks} />}
                </Show>
            )}
        </For>
    );
}

/** Write what tells a block from its siblings, drawn anew as another type. */
function keyOf({ key, type }: Block): string {
    return `${type} ${key}`;
}

/** Group blocks by the lists they are the items of. */
function groupsOf(blocks: readonly Block[]): Group[] {
    const groups: (Group & { readonly blocks: Block[] })[] = [];
    for (const block of blocks) {
        const list = listOf(block);
        const last = groups.at(-1);
        if (last !== undefined && list !== undefined && last.list === list) {
            last.blocks.push(block);
        } else {
            groups.push({
                key: keyOf(block),
                ...(list === undefined ? {} : { list }),
                blocks: [block],
            });
        }
    }

    return groups;
}

/** Read the list a block is an item of, undefined for any other block. */
function listOf({ type, value }: Block): Group["list"] {
    if (type === "todo") {
        return "todo";
    }
    const isNumbered = Item.safeParse(value).data?.style === "number";

    return type === "list" ? (isNumbered ? "number" : "bullet") : undefined;
}

/** Render a list and its items. */
function List(properties: {
    /** The list's element and look. */
    readonly list: NonNullable<Group["list"]>;
    /** The items. */
    readonly blocks: readonly Block[];
}): JSX.Element {
    const items = () => (
        <For each={properties.blocks} keyed={keyOf}>
            {(block) => <BlockView block={block()} />}
        </For>
    );
    const start = () => Item.safeParse(properties.blocks[0]?.value).data?.start;

    return (
        <Show
            when={properties.list === "number"}
            fallback={
                <ul {...style.attrs(properties.list === "todo" && styles.todos)}>{items()}</ul>
            }
        >
            <ol start={start()}>{items()}</ol>
        </Show>
    );
}

/** Render a block by its type, by the owner's component where it has one. */
function BlockView(properties: {
    /** The block. */
    readonly block: Block;
}): JSX.Element {
    // read how its type is drawn, and what it has
    const { components, reveal } = useContext(RichTextContext);
    const isNew = reveal.isWaiting() || reveal.isFading();
    const type = untrack(() => properties.block.type);
    const own = untrack(() => components().blocks?.[type]);
    const content = () => (
        <>
            <Spans spans={properties.block.spans} />
            <Blocks blocks={properties.block.blocks} />
        </>
    );

    // draw it as its owner does, else by its type
    if (own !== undefined) {
        return createComponent(own, {
            get block() {
                return properties.block;
            },
            get children() {
                return content();
            },
        });
    }
    switch (type) {
        case "heading": {
            const level = () => Heading.safeParse(properties.block.value).data?.level ?? 1;
            const element = dynamic(() => HEADINGS[level() - 1] ?? "h1");

            return createComponent(element, {
                get children() {
                    return content();
                },
            });
        }
        case "list":
            return <li {...style.attrs(reveal.isWaiting() && styles.marked)}>{content()}</li>;
        case "todo":
            return <TodoView block={properties.block} />;
        case "quote":
            return (
                <blockquote {...style.attrs(reveal.isWaiting() && styles.marked)}>
                    {content()}
                </blockquote>
            );
        case "codeBlock":
            return <CodeView block={properties.block} isNew={isNew} />;
        case "divider":
            return <hr {...style.attrs(isNew && styles.fading)} />;
        case "image":
            return <ImageView block={properties.block} />;
        case "table":
            return <TableView block={properties.block} />;
        default:
            return (
                <>
                    <p>
                        <Spans spans={properties.block.spans} />
                    </p>
                    <Blocks blocks={properties.block.blocks} />
                </>
            );
    }
}

/** The elements of headings by their levels. */
const HEADINGS = ["h1", "h2", "h3", "h4", "h5", "h6"] as const;

/** Render a todo: its box, which people check where the owner handles it, and its text. */
function TodoView(properties: {
    /** The block. */
    readonly block: Block;
}): JSX.Element {
    // name the box by the text, and leave it alone without a handler
    const { onTodoChange, reveal } = useContext(RichTextContext);
    const id = createUniqueId();
    const handler = () => onTodoChange();

    return (
        <li {...style.attrs(styles.todo, reveal.isWaiting() && styles.marked)}>
            <Checkbox
                aria-labelledby={id}
                checked={Todo.safeParse(properties.block.value).data?.isChecked === true}
                readOnly={handler() === undefined}
                onCheckedChange={(isChecked) => handler()?.(properties.block, isChecked)}
            />
            <div {...style.attrs(styles.todoText)}>
                <span id={id}>
                    <Spans spans={properties.block.spans} />
                </span>
                <Blocks blocks={properties.block.blocks} />
            </div>
        </li>
    );
}

/** Render a block of code in the colors of its language, which a button copies. */
function CodeView(properties: {
    /** The block. */
    readonly block: Block;
    /** Whether it comes up as it is written. */
    readonly isNew: boolean;
}): JSX.Element {
    // find the grammar of its language
    const { grammar, isStreaming } = useContext(RichTextContext);
    const locale = useLocale();
    const language = () => Code.safeParse(properties.block.value).data?.language;
    const colored = () => {
        const name = language();

        return name === undefined ? undefined : grammar(name);
    };

    return (
        <CodeBlock xstyle={properties.isNew && styles.fading}>
            <CodeBlockHeader>
                <CodeBlockTitle>{language() ?? locale.render(t`Code`)}</CodeBlockTitle>
                <CopyButton
                    value={plainOf(properties.block.spans)}
                    label={locale.render(t`Copy code`)}
                    size="icon-sm"
                    disabled={isStreaming()}
                />
            </CodeBlockHeader>
            <CodeBlockContent
                code={plainOf(properties.block.spans)}
                grammar={colored()}
                label={language() ?? locale.render(t`Code`)}
            />
        </CodeBlock>
    );
}

/** Render an image that may load, else its words as a link to it. */
function ImageView(properties: {
    /** The block. */
    readonly block: Block;
}): JSX.Element {
    const { allowsImage } = useContext(RichTextContext);
    const image = () => Image.safeParse(properties.block.value).data;

    return (
        <Show when={image()}>
            {(shown) => (
                <Show
                    when={allowsImage(shown().source)}
                    fallback={
                        <p>
                            <Anchor href={shown().source}>
                                {shown().alternative === "" ? shown().source : shown().alternative}
                            </Anchor>
                        </p>
                    }
                >
                    <img src={shown().source} alt={shown().alternative} />
                </Show>
            )}
        </Show>
    );
}

/** Render a table, its first row heading it. */
function TableView(properties: {
    /** The block. */
    readonly block: Block;
}): JSX.Element {
    // read how the columns align, and the rows apart from the header
    const alignments = () => Columns.safeParse(properties.block.value).data?.alignments ?? [];
    const { reveal } = useContext(RichTextContext);
    const revealed = () => (reveal.isWaiting() || reveal.isFading()) && styles.fading;
    const rows = (isHeader: boolean) =>
        properties.block.blocks.filter(
            ({ value }) => (Row.safeParse(value).data?.isHeader ?? false) === isHeader,
        );
    const aligned = (index: number) => styles.align(alignments()[index] ?? "start");

    return (
        <Table>
            <TableHeader>
                <For each={rows(true)} keyed={(row) => row.key}>
                    {(row) => (
                        <TableRow xstyle={revealed()}>
                            <For each={row().blocks} keyed={(cell) => cell.key}>
                                {(cell, index) => (
                                    <TableHead xstyle={aligned(index())}>
                                        <Spans spans={cell().spans} />
                                    </TableHead>
                                )}
                            </For>
                        </TableRow>
                    )}
                </For>
            </TableHeader>
            <TableBody>
                <For each={rows(false)} keyed={(row) => row.key}>
                    {(row) => (
                        <TableRow xstyle={revealed()}>
                            <For each={row().blocks} keyed={(cell) => cell.key}>
                                {(cell, index) => (
                                    <TableCell xstyle={aligned(index())}>
                                        <Spans spans={cell().spans} />
                                    </TableCell>
                                )}
                            </For>
                        </TableRow>
                    )}
                </For>
            </TableBody>
        </Table>
    );
}

/** Render spans, each with its marks, in their places as they change. */
function Spans(properties: {
    /** The spans. */
    readonly spans: readonly TextSpan[];
}): JSX.Element {
    return (
        <For each={properties.spans} keyed={false}>
            {(span) => <For each={[shapeOf(span())]}>{() => <SpanView span={span()} />}</For>}
        </For>
    );
}

/** Write what a span is apart from its text: its marks, or its node. */
function shapeOf(span: TextSpan): string {
    return JSON.stringify("text" in span ? (span.marks ?? {}) : span.node);
}

/** Render a span: its text within its marks, or its node. */
function SpanView(properties: {
    /** The span, whose marks or node stay the same. */
    readonly span: TextSpan;
}): JSX.Element {
    // draw a node as its owner does, else by its type
    const { components } = useContext(RichTextContext);
    const span = untrack(() => properties.span);
    if (!("text" in span)) {
        const { type, value } = span.node;
        const own = untrack(() => components().nodes?.[type]);

        return own === undefined ? `@${type}` : createComponent(own, { value });
    }

    // put the text in its marks, the outermost first
    const text = () => ("text" in properties.span ? properties.span.text : "");
    const marks = MARKS.filter((type) => span.marks?.[type] !== undefined);
    const others = Object.keys(span.marks ?? {}).filter((type) => !MARKS.includes(type));
    const drawn = [...others, ...marks].reduceRight<() => JSX.Element>(
        (inner, type) => () => (
            <MarkView type={type} value={span.marks?.[type] ?? null}>
                {inner()}
            </MarkView>
        ),
        () => <Growing text={text()} />,
    );

    return drawn();
}

/** A stretch of text, and how it comes up. */
interface Stretch {
    /** The text. */
    readonly text: string;
    /** Whether it is there at once, fades in, or waits for its line. */
    readonly entry: "shown" | "fading" | "waiting";
}

/** Render text that grows at its end, leaving what is there as it is. */
function Growing(properties: {
    /** The text. */
    readonly text: string;
}): JSX.Element {
    // add what is new as stretches, and start over where what was there changes
    const { reveal } = useContext(RichTextContext);
    const stretches = createMemo<readonly Stretch[]>((before = []) => {
        // keep the stretches of a same text
        const shown = before.map((stretch) => stretch.text).join("");
        const { text } = properties;
        if (text === shown) {
            return before;
        }

        // split what waits for its line into words, which the line breaks between
        const isGrowing = text.startsWith(shown);
        const added = isGrowing ? text.slice(shown.length) : text;
        const isWaiting = reveal.isWaiting();
        const entry = isWaiting ? "waiting" : reveal.isFading() ? "fading" : "shown";
        const words = isWaiting ? (added.match(/\S+\s*|\s+/gu) ?? []) : [added];

        return [
            ...(isGrowing ? before : []),
            ...words.map((word) => ({ text: word, entry }) as const),
        ];
    });

    return (
        <For each={stretches()}>
            {(stretch) =>
                stretch.entry === "shown" ? (
                    <Lines text={stretch.text} />
                ) : (
                    <span
                        data-slot="rich-text-stretch"
                        ref={(element) => {
                            if (stretch.entry === "waiting") {
                                reveal.add(element);
                            }
                        }}
                        {...style.attrs(
                            stretch.entry === "waiting" ? styles.waiting : styles.fading,
                        )}
                    >
                        <Lines text={stretch.text} />
                    </span>
                )
            }
        </For>
    );
}

/** The kit's marks, the outermost first. */
const MARKS: readonly string[] = ["link", "bold", "italic", "strike", "code"];

/** Render text, its line breaks as breaks. */
function Lines(properties: {
    /** The text. */
    readonly text: string;
}): JSX.Element {
    return (
        <For each={properties.text.split("\n")}>
            {(line, index) => (
                <>
                    <Show when={index() > 0}>
                        <br />
                    </Show>
                    {line}
                </>
            )}
        </For>
    );
}

/** Render a mark around a text, by the owner's component where it has one. */
function MarkView(properties: {
    /** The mark's type. */
    readonly type: string;
    /** The mark's value. */
    readonly value: JsonValue;
    /** The text. */
    readonly children: JSX.Element;
}): JSX.Element {
    // read how its type is drawn
    const { components } = useContext(RichTextContext);
    const type = untrack(() => properties.type);
    const own = untrack(() => components().marks?.[type]);

    // draw it as its owner does, else by its type
    if (own !== undefined) {
        return createComponent(own, {
            get value() {
                return properties.value;
            },
            get children() {
                return properties.children;
            },
        });
    }
    switch (type) {
        case "bold":
            return <strong>{properties.children}</strong>;
        case "italic":
            return <em>{properties.children}</em>;
        case "strike":
            return <s>{properties.children}</s>;
        case "code":
            return <code>{properties.children}</code>;
        case "link":
            return (
                <Anchor href={Link.safeParse(properties.value).data?.href ?? ""}>
                    {properties.children}
                </Anchor>
            );
        default:
            return properties.children;
    }
}

/** Render a link to an address people open, any other as its text. */
function Anchor(properties: {
    /** The address. */
    readonly href: string;
    /** The text. */
    readonly children: JSX.Element;
}): JSX.Element {
    return (
        <Show when={ADDRESSES.test(properties.href)} fallback={properties.children}>
            <a
                href={properties.href}
                {...(OUTSIDE.test(properties.href)
                    ? { target: "_blank", rel: "noopener noreferrer" }
                    : {})}
            >
                {properties.children}
            </a>
        </Show>
    );
}

/** Read the text of spans alone. */
function plainOf(spans: readonly TextSpan[]): string {
    return spans.map((span) => ("text" in span ? span.text : "")).join("");
}
