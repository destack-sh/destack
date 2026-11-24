import { present } from "@destack/schema";
import type { Grammar } from "../grammar/index.ts";

/** The languages, and the files and Markdown code blocks each is found by. */
const LANGUAGES = [
    {
        name: "bash",
        title: "Bash",
        tags: ["sh", "shell", "zsh"],
        extensions: ["sh", "bash", "zsh"],
        files: [".bashrc", ".zshrc", ".profile"],
    },
    { name: "c", title: "C", tags: ["h"], extensions: ["c", "h"], files: [] },
    {
        name: "cpp",
        title: "C++",
        tags: ["c++", "cc"],
        extensions: ["cpp", "cc", "cxx", "hpp", "hh", "hxx"],
        files: [],
    },
    { name: "csharp", title: "C#", tags: ["cs", "c#"], extensions: ["cs"], files: [] },
    { name: "css", title: "CSS", tags: [], extensions: ["css"], files: [] },
    { name: "diff", title: "Diff", tags: ["patch"], extensions: ["diff", "patch"], files: [] },
    {
        name: "dockerfile",
        title: "Dockerfile",
        tags: ["docker"],
        extensions: ["dockerfile"],
        files: ["Dockerfile", "Containerfile"],
    },
    { name: "go", title: "Go", tags: ["golang"], extensions: ["go"], files: [] },
    { name: "html", title: "HTML", tags: ["htm"], extensions: ["html", "htm"], files: [] },
    {
        name: "ini",
        title: "INI",
        tags: ["properties"],
        extensions: ["ini", "cfg", "conf", "properties", "env"],
        files: [".gitconfig", ".editorconfig", ".npmrc", ".env"],
    },
    { name: "java", title: "Java", tags: [], extensions: ["java"], files: [] },
    {
        name: "javascript",
        title: "JavaScript",
        tags: ["js", "jsx", "mjs", "cjs"],
        extensions: ["js", "jsx", "mjs", "cjs"],
        files: [],
    },
    {
        name: "json",
        title: "JSON",
        tags: ["jsonc", "json5"],
        extensions: ["json", "jsonc", "json5", "webmanifest", "map"],
        files: [],
    },
    { name: "kotlin", title: "Kotlin", tags: ["kt"], extensions: ["kt", "kts"], files: [] },
    {
        name: "markdown",
        title: "Markdown",
        tags: ["md", "mdx"],
        extensions: ["md", "mdx", "markdown"],
        files: [],
    },
    { name: "php", title: "PHP", tags: [], extensions: ["php"], files: [] },
    { name: "python", title: "Python", tags: ["py"], extensions: ["py", "pyi"], files: [] },
    {
        name: "ruby",
        title: "Ruby",
        tags: ["rb"],
        extensions: ["rb", "gemspec", "rake"],
        files: ["Gemfile", "Rakefile"],
    },
    { name: "rust", title: "Rust", tags: ["rs"], extensions: ["rs"], files: [] },
    { name: "scss", title: "SCSS", tags: ["sass"], extensions: ["scss"], files: [] },
    {
        name: "sql",
        title: "SQL",
        tags: ["postgres", "postgresql", "sqlite"],
        extensions: ["sql"],
        files: [],
    },
    { name: "swift", title: "Swift", tags: [], extensions: ["swift"], files: [] },
    { name: "toml", title: "TOML", tags: [], extensions: ["toml"], files: [] },
    { name: "tsx", title: "TSX", tags: [], extensions: ["tsx"], files: [] },
    {
        name: "typescript",
        title: "TypeScript",
        tags: ["ts", "mts", "cts"],
        extensions: ["ts", "mts", "cts"],
        files: [],
    },
    {
        name: "xml",
        title: "XML",
        tags: ["svg", "rss"],
        extensions: ["xml", "svg", "plist", "xsd"],
        files: [],
    },
    { name: "yaml", title: "YAML", tags: ["yml"], extensions: ["yaml", "yml"], files: [] },
] as const;

/** The name of a language, such as `typescript`. */
export type LanguageName = (typeof LANGUAGES)[number]["name"];

/** A language code is written in. */
export class Language {
    /** Every language, in the order of their names. */
    static readonly all: readonly Language[] = LANGUAGES.map((each) => new Language(each));

    /** The name. */
    readonly name: LanguageName;
    /** The title people read. */
    readonly title: string;
    /** The tags it has besides its name, such as `ts`. */
    readonly tags: readonly string[];
    /** The extensions of its files, without their dots. */
    readonly extensions: readonly string[];
    /** The names of its files that go without an extension. */
    readonly files: readonly string[];

    /** Describe a language. */
    private constructor(description: (typeof LANGUAGES)[number]) {
        // take what the list has of it
        this.name = description.name;
        this.title = description.title;
        this.tags = description.tags;
        this.extensions = description.extensions;
        this.files = description.files;
    }

    /** Read the language of a name. */
    static of(name: LanguageName): Language {
        return present(
            Language.all.find((language) => language.name === name),
            `the language '${name}'`,
        );
    }

    /** Find the language of a file by its name or path, absent when not known. */
    static ofFile(path: string): Language | undefined {
        // compare the whole name first, then what follows its last point
        const file = path.slice(path.lastIndexOf("/") + 1);
        const extension = file.slice(file.lastIndexOf(".") + 1).toLowerCase();

        return Language.all.find(
            ({ files, extensions }) => files.includes(file) || extensions.includes(extension),
        );
    }

    /** Find the language of a tag, such as a Markdown code block's `ts`, absent when not known. */
    static ofTag(tag: string): Language | undefined {
        const word = tag.toLowerCase();

        return Language.all.find(({ name, tags }) => name === word || tags.includes(word));
    }

    /** Load the grammar, which bundlers split into one chunk per language. */
    async load(): Promise<Grammar> {
        // import its module, and require the grammar it exports
        const module: unknown = await import(`../grammar/language/${this.name}.ts`);
        if (!isGrammarModule(module)) {
            throw new TypeError(`grammar module of '${this.name}' is invalid: it has no grammar`);
        }

        return module.default;
    }
}

/** Check that a value is a grammar's module, which always exports its grammar by default. */
function isGrammarModule(value: unknown): value is { readonly default: Grammar } {
    return (
        typeof value === "object" &&
        value !== null &&
        "default" in value &&
        typeof value.default === "object"
    );
}
