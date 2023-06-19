import { graphql } from "@/gql";
import {
  StatementType,
  type File,
  type Project,
  type ProjectVersion,
  type Scalars,
  type Field,
  type Statement,
  type Record as BRecord,
} from "@/gql/graphql";
import {
  CONTENT_MARGIN_X_NARROW,
  CONTENT_MARGIN_X_WIDE,
  CONTENT_WIDTH_NARROW,
  CONTENT_WIDTH_WIDE,
  useAppearanceState,
  type EditorAppearance,
  type Theme,
} from "@/state/appearance";
import type { ModuleIndex } from "@/state/module";
import { randomHexString } from "@/utils/functools";
import {
  ArrowLeftIcon,
  ArrowRightIcon,
  ArrowsPointingInIcon,
  ArrowsPointingOutIcon,
  XCircleIcon,
} from "@heroicons/vue/24/outline";
import { useApolloClient } from "@vue/apollo-composable";
import { useElementBounding } from "@vueuse/core";
import { defineStore } from "pinia";
import { computed, inject, onBeforeUnmount, provide, ref, type Ref } from "vue";

export type ProjectHeader = Pick<Project, "id" | "name" | "slug" | "canWrite" | "createdAt" | "updatedAt">;
export type ProjectVersionHeader = Pick<
  ProjectVersion,
  "id" | "name" | "description" | "createdAt" | "committed" | "committedAt"
>;
export type FileHeader = Pick<
  File,
  "__typename" | "id" | "name" | "createdAt" | "updatedAt" | "deletedAt" | "directory"
>;
export type StatementHeader = Pick<
  Statement,
  | "__typename"
  | "id"
  | "modifier"
  | "type"
  | "name"
  | "createdAt"
  | "updatedAt"
  | "deletedAt"
  | "orderKey"
  | "commented"
  | "parent"
>;

export type ViewId = "explorer" | "search" | "history" | "issues" | "comments" | "environment" | "instruction";

export type EditorType = "file" | "statement" | "launch";

const BENCH_STATE_VERSION = 2;

export function prettifySlug(path: string) {
  // replace non-URL friendly characters with dashes
  return path.replace(/[^a-zA-Z0-9-_./@:]/g, "-");
}

// note that editor state should be JSON serializable (except below)
const UNSERIALIZABLE_EDITOR_PROPS = ["_bench", "_context"];
export abstract class Editor {
  type: EditorType;
  id: string;
  name: string;
  path: string;
  groupId: string | null = null; // id instead of EditorGroup to avoid circular dependency
  appearance: EditorAppearance = {};
  // refs assigned on creation/component instantiation
  _bench: ReturnType<typeof useBenchState> | undefined = undefined;
  _context: any | undefined = undefined;

  constructor(type: EditorType, id: string, name: string, path: string, groupId: string | null = null) {
    this.name = name;
    this.type = type;
    this.id = id;
    this.path = path;
    this.groupId = groupId;
  }

  copy(): Editor {
    // serialize, deserialize and reset id
    const serialized = JSON.stringify(stripEditor(this));
    const copy = instantiate(JSON.parse(serialized), this.bench);
    copy.resetId();
    return copy;
  }

  abstract resetId(): void;

  static parsePath(path: string, module: ModuleIndex): Editor | null {
    throw new Error("not implemented"); // implement per editor type
  }

  get hasWhiteBackground() {
    return true;
  }

  get effectiveWide() {
    return this.appearance?.wide ?? this.bench.appearance.contentWide;
  }

  get contentWidth() {
    if (this.appearance?.wide === false) {
      return CONTENT_WIDTH_NARROW;
    } else if (this.appearance?.wide === true) {
      return CONTENT_WIDTH_WIDE;
    } else {
      return this.bench.appearance.contentWidth;
    }
  }

  get contentMarginX() {
    if (this.appearance?.wide === false) {
      return CONTENT_MARGIN_X_NARROW;
    } else if (this.appearance?.wide === true) {
      return CONTENT_MARGIN_X_WIDE;
    } else {
      return this.bench.appearance.contentMarginX;
    }
  }

  // :ContentSizeProps
  get contentWidthWithMargin() {
    return this.contentWidth + 2 * this.contentMarginX;
  }
  get contentWidthAsFixed() {
    return {
      width: `${this.contentWidth}px`,
    };
  }
  get contentWidthAsMaxWidth() {
    return {
      maxWidth: `${this.contentWidth}px`,
    };
  }
  get contentMarginXAsPaddingX() {
    return {
      paddingLeft: `${this.contentMarginX}px`,
      paddingRight: `${this.contentMarginX}px`,
    };
  }
  get contentMarginXAsMarginX() {
    return {
      marginLeft: `${this.contentMarginX}px`,
      marginRight: `${this.contentMarginX}px`,
    };
  }

  get focused() {
    return this._bench?.focusedEditorId == this.id;
  }

  get group(): EditorGroup | undefined {
    if (this.groupId == null) {
      return undefined;
    }
    return this.bench.groups.find((g) => g.id == this.groupId);
  }

  get bench(): ReturnType<typeof useBenchState> {
    if (this._bench == null) {
      throw new Error(`editor ${this.id} has no bench`);
    }
    return this._bench;
  }

  get context(): any {
    // TODO @Cleanup: this should be typed but TS throws up
    if (this._context == null) {
      throw new Error(`editor ${this.id} has no context`);
    }
    return this._context;
  }

  onDeserialized(bench: ReturnType<typeof useBenchState>) {
    this._bench = bench;
  }

  onMounted(context: EditorContext<any>) {
    if (this._context != null) {
      throw new Error(`editor ${this.id} already has a context`);
    }
    this._context = context;
  }

  onUnmounted() {
    this._context = undefined;
  }

  blur() {
    // noop by default
  }
}

export type EditorGroup = {
  id: string;
  name: string;
  editors: Editor[];
  activeEditorId: string | null;
};

function makeEditorGroup(id: string, name: string): EditorGroup {
  return {
    id,
    name,
    editors: [],
    activeEditorId: null,
  };
}

export const useBenchState = defineStore("bench", {
  state: () => {
    return {
      version: BENCH_STATE_VERSION,
      // bench
      projectId: null as string | null,
      projectVersionId: null as string | null,
      readonly: false,
      // views
      activeViewId: "explorer" as ViewId,
      focusedViewId: null as ViewId | null,
      // editors
      left: makeEditorGroup("left", "Left"),
      right: makeEditorGroup("right", "Right"),
      focusedEditorId: null as string | null,
      // appearance/settings (should be merged into appearance? but is bench specific...)
      debug: false,
      showGenerated: true,
      showLineNumbers: false,
      showEditorGroupHeader: false,
      showGlobalHeader: true,
      showViewSelection: true,
      showViewContent: false,
      zenMode: false,
    };
  },
  getters: {
    groups(state) {
      return [state.left, state.right];
    },
    group(): (id: string) => EditorGroup {
      return (id: string) => {
        const group = this.groups.find((g) => g.id == id);
        if (group == null) throw new Error(`editor group ${id} not found`);
        return group;
      };
    },
    editors(state) {
      return state.left.editors.concat(state.right.editors);
    },
    focusedEditor(): Editor | undefined {
      return this.editors.find((e) => e.id == this.focusedEditorId);
    },
    focusedFileId(): string | null {
      return this.focusedEditor?.type == "file" ? (this.focusedEditor as FileEditor).fileId : null;
    },
    focusedFile(): FileEditor | undefined {
      return this.focusedEditor?.type == "file" ? (this.focusedEditor as FileEditor) : undefined;
    },
    focusedStatementId(): string | null {
      return this.focusedFile?.activeStatementId ?? null;
    },
    focusedGroup(): EditorGroup | undefined {
      if (this.focusedEditor?.groupId == null) return undefined;
      return this.group(this.focusedEditor?.groupId);
    },
    appearance() {
      return useAppearanceState();
    },
    theme(): Theme {
      const appearance = useAppearanceState();
      return appearance.theme;
    },
    textSmall(): boolean {
      const appearance = useAppearanceState();
      return appearance.textSmall;
    },
    fullscreen(): boolean {
      const appearance = useAppearanceState();
      return appearance.fullscreen;
    },
    fontMono(): boolean {
      const appearance = useAppearanceState();
      return appearance.fontMono;
    },
    inlineMetrics(): boolean {
      const appearance = useAppearanceState();
      return appearance.inlineMetrics;
    },
  },
  actions: {
    setProject(projectId: string, versionId: string): void {
      this.$reset();
      this.projectId = projectId;
      this.projectVersionId = versionId;
    },

    // views

    setActiveView(viewId: ViewId): void {
      this.activeViewId = viewId;
    },

    openActiveView(viewId: ViewId): void {
      this.activeViewId = viewId;
      this.showViewContent = true;
    },

    focusView(viewId: ViewId): void {
      if (viewId == this.focusedViewId) return;
      this.focusedViewId = viewId;
      this.openActiveView(viewId);
      this.blur();
      console.debug(`focus view ${viewId}`);
    },

    blurView(viewId?: ViewId): void {
      if (viewId != null && viewId != this.focusedViewId) return;
      this.focusedViewId = null;
      console.debug(`blur view ${viewId}`);
    },

    // editors

    _removeEditorFromGroup(editor: Editor): void {
      if (editor.groupId == null) return;
      const group = this.group(editor.groupId);
      if (group == null) return;
      group.editors = group.editors.filter((e) => e != editor);
      if (group.activeEditorId == editor.id) {
        // if active editor was removed, set first editor as active
        group.activeEditorId = group.editors[0]?.id || null;
        // if editor was focused, focus new active editor
        if (editor.id == this.focusedEditorId) {
          this.focusedEditorId = group.activeEditorId;
        }
      }
      editor.groupId = null;
    },

    openEditor(editor: Editor, group?: EditorGroup): Editor {
      // if group wasn't passed, just return the editor if it's already open
      if (group == null && editor.groupId != null) {
        return editor;
      }
      console.log(`open editor ${editor.path} in group ${group?.id}`);
      group = group || this.focusedGroup || this.left;
      // change editor group if different
      if (editor.groupId != group.id) {
        if (editor.groupId != null) {
          // remove from old group
          this._removeEditorFromGroup(editor);
        }
        editor.groupId = group.id;
        group.editors.push(editor);
      }
      return editor;
    },

    closeEditor(editor: Editor): void {
      console.log(`close editor ${editor.path}`);
      if (editor.groupId != null) {
        this._removeEditorFromGroup(editor);
      }
    },

    closeEditorGroup(group: EditorGroup): void {
      console.log(`close editor group ${group.id}`);
      group.editors.forEach((e) => this.closeEditor(e));
    },

    moveEditor(editor: Editor, group: EditorGroup, options?: { copy?: boolean }): void {
      const wasFocused = editor == this.focusedEditor;
      if (options?.copy) {
        editor = editor.copy();
      }
      this.openEditor(editor, group);
      if (wasFocused) {
        this.focusEditor(editor);
      }
    },

    openFile(
      file: { id: string; name: string },
      options?: { group?: EditorGroup; create?: boolean; focus?: boolean }
    ): Editor {
      let editor = this.editors.find((e) => e.type == "file" && (e as FileEditor).fileId == file.id);
      if (!editor || options?.create) {
        console.log(`create new file editor for ${file.id} ${file.name}`);
        editor = new FileEditor(file);
        editor.onDeserialized(this);
      }
      this.openEditor(editor, options?.group);
      if (options?.focus) {
        this.focusEditor(editor);
      }
      return editor;
    },

    openStatement(
      statement: { id: string; name?: string | null },
      options?: { group?: EditorGroup; create?: boolean; focus?: boolean }
    ): Editor {
      let editor = this.editors.find(
        (e) => e.type == "statement" && (e as StatementEditor).statementId == statement.id
      );
      if (!editor || options?.create) {
        console.log(`create new statement editor for ${statement.name}`);
        editor = new StatementEditor(statement);
        editor.appearance.wide = true; // default to wide
        editor.onDeserialized(this);
      }
      this.openEditor(editor, options?.group);
      if (options?.focus) {
        this.focusEditor(editor);
      }
      return editor;
    },

    openRun(
      statement: { id: string; name?: string | null },
      options?: { group?: EditorGroup; create?: boolean; focus?: boolean }
    ): Editor {
      let editor = this.editors.find((e) => e.type == "launch" && (e as LaunchEditor).statementId == statement.id);
      if (!editor || options?.create) {
        console.log(`create new launch editor for ${statement.name}`);
        editor = new LaunchEditor(statement);
        editor.onDeserialized(this);
      }
      this.openEditor(editor, options?.group);
      if (options?.focus) {
        this.focusEditor(editor);
      }
      return editor;
    },

    nextGroup(group: EditorGroup): EditorGroup | undefined {
      const index = this.groups.indexOf(group);
      return this.groups[(index + 1) % this.groups.length];
    },

    focusGroup(group: EditorGroup): void {
      if (this.focusedGroup?.id == group.id) return;
      if (group.activeEditorId == null) {
        throw new Error("group must have an active editor");
      }
      console.debug(`focus group ${group.id}`);
      this.focusedEditorId = group.activeEditorId;
    },

    focusEditor(editor: Editor): void {
      this.focusedViewId = null;
      if (this.focusedEditor?.id == editor.id) return;
      console.debug(`focus editor ${editor.path} in group ${editor.groupId}`);
      if (!editor.groupId) {
        throw new Error("editor must be in a group: " + editor.path);
      }
      // blur all other editors
      this.editors.filter((e) => e.id != editor.id).forEach((e) => e.blur());
      this.focusedEditorId = editor.id;
      this.group(editor.groupId).activeEditorId = editor.id;
    },

    focusFile(file: FileHeader, group?: EditorGroup): Editor {
      const editor = this.openFile(file, { group });
      this.focusEditor(editor);
      return editor;
    },

    focusStatement(statement: StatementHeader, group?: EditorGroup): Editor {
      const editor = this.openStatement(statement, { group });
      this.focusEditor(editor);
      return editor;
    },

    blur() {
      this.editors.forEach((e) => e.blur());
    },

    // settings

    setZenMode(zenMode: boolean) {
      const appearance = useAppearanceState();
      this.zenMode = zenMode;
      this.showViewContent = !zenMode;
      this.showGlobalHeader = !zenMode;
      appearance.fullscreen = zenMode;
    },

    // migration

    _doMigrateTo(versionId: string, refMappings: Record<string, string>): void {
      // migrate by serializing state and replacing refs
      let stateJson = benchStateToJson(this);
      // TODO @Performance: replace editor refs on migration in a single pass
      for (const [sourceId, targetId] of Object.entries(refMappings)) {
        // replace all matches of ref.source with ref.target
        // (need to use regex to replace *all* matches)
        const re = new RegExp(`"${sourceId}"`, "g");
        stateJson = stateJson.replace(re, `"${targetId}"`);
      }
      this.$reset();
      benchInitFromJson(this, stateJson);

      // remove editors with refs we don't have anymore
      // note that this also closes any module-external refs
      // I tried to fix this by only removing refs we _used_ tohave (checking for original ref.target)
      // but that doesn't work for refs that were just created in first source version.
      const targetRefs = Object.values(refMappings);
      for (const editor of this.editors) {
        let editorRef = null;
        if (editor.type == "file") {
          editorRef = (editor as FileEditor).fileId;
        }
        if (editorRef != null && !targetRefs.includes(editorRef)) {
          console.debug(`close outdated editor ${editor.path} (${editor.id} pointed to ${editorRef})`);
          this.closeEditor(editor);
        }
      }
      this.projectVersionId = versionId;
    },
  },
});

// persistence

function stripEditor(editor: Editor) {
  const stripped = { ...editor };
  for (const prop of UNSERIALIZABLE_EDITOR_PROPS) {
    delete (stripped as any)[prop];
  }
  return stripped;
}

function benchStateToJson(bench: ReturnType<typeof useBenchState>): string {
  // clean up editor state for serialization
  const state = {
    ...bench.$state,
    left: {
      ...bench.$state.left,
      editors: bench.$state.left.editors.map((e) => stripEditor(e)),
    },
    right: {
      ...bench.$state.right,
      editors: bench.$state.right.editors.map((e) => stripEditor(e)),
    },
  };
  return JSON.stringify(state);
}

function benchInitFromJson(bench: ReturnType<typeof useBenchState>, state: string) {
  bench.$reset();
  bench.version = -1;
  bench.$patch(JSON.parse(state));
  // version check
  // TODO @Robustness: improve Bench state versioning
  if (bench.version != BENCH_STATE_VERSION) {
    const version = bench.version;
    bench.$reset(); // reset to initial state
    throw new Error(`Bench state version mismatch (${version} != ${BENCH_STATE_VERSION})`);
  }
  // instantiate editors
  for (const group of bench.groups) {
    group.editors = group.editors.map((e) => instantiate(e, bench));
  }
}

export function useBenchPersistence(intervalMs = 1000) {
  const bench = useBenchState();

  function save(projectId?: string) {
    if (bench.projectId == null) return;
    if (projectId != null && bench.projectId != projectId) {
      throw new Error(`cannot save editor state for project ${projectId} (current project is ${bench.projectId})`);
    }
    localStorage.setItem(`bench-state-${bench.projectId}`, benchStateToJson(bench));
  }

  function load(projectId: string): boolean {
    if (bench.projectId != projectId) {
      bench.projectId = projectId;
      bench.projectVersionId = null;
    }
    const state = localStorage.getItem(`bench-state-${projectId}`);
    if (state) {
      try {
        benchInitFromJson(bench, state);
        console.log(`restored bench state for project ${projectId}`);
        return true;
      } catch (e) {
        console.error(`unable to restore bench state for project ${projectId}`, e);
      }
    }
    return false;
  }

  // save every interval
  const interval = setInterval(save, intervalMs);
  onBeforeUnmount(() => {
    save();
    clearInterval(interval);
  });

  return { save, load };
}

// migration

export function useBenchMigrations() {
  const bench = useBenchState();
  const client = useApolloClient();
  const migratingTo = ref<string | null>(null);

  async function migrateTo(projectId: string, sourceVersionId: string, targetVersionId: string): Promise<boolean> {
    migratingTo.value = targetVersionId;
    console.log(`migrate bench ${projectId} from ${sourceVersionId} to ${targetVersionId}`);
    // get all ref mappings
    const ret = await client.client.query({
      query: graphql(/* GraphQL */ `
        query projectMigrationRefs($projectId: GlobalID!, $sourceVersionId: GlobalID!, $targetVersionId: GlobalID!) {
          project(id: $projectId) {
            migrationMappings(sourceVersionId: $sourceVersionId, targetVersionId: $targetVersionId) {
              isReverse
              sourceVersion {
                id
                createdAt
                tag
                name
              }
              targetVersion {
                id
                createdAt
                tag
                name
              }
              refMappings {
                sourceId
                sourceVersionId
                targetId
                targetVersionId
              }
            }
          }
        }
      `),
      variables: {
        projectId,
        sourceVersionId,
        targetVersionId,
      },
    });
    migratingTo.value = null;
    if (ret.error != null || ret.data?.project?.migrationMappings == null) {
      console.error("unable to migrate, error getting intermediate refs", ret.error);
      return false;
    }
    // apply migrations
    const migrationMappings = ret.data?.project.migrationMappings;
    const refMappings: Record<string, string> = {};
    for (const mapping of migrationMappings.refMappings) {
      refMappings[mapping.sourceId] = mapping.targetId;
    }
    bench._doMigrateTo(targetVersionId, refMappings);
    console.log(`migrated bench  ${migrationMappings?.sourceVersion.id} to ${migrationMappings?.targetVersion.id}`);
    return true;
  }

  return {
    migrating: computed(() => migratingTo.value != null),
    migrateTo,
  };
}

// context

export type EditorContext<T extends Editor> = {
  editor: Ref<T>;
  component: Ref<any>;
  container: Ref<HTMLElement | null>;
  size: Ref<{ width: number; height: number }>;
  pos: Ref<{ left: number; top: number }>;
  actions: Ref<EditorAction[]>;
  actionGroups?: Ref<ActionGroup[]>;
};

export const EDITOR_CONTEXT = "__editor__";

export function provideEditorContext<T extends Editor>(
  editor: Ref<T>,
  component: Ref<any>,
  container: Ref<HTMLElement | null>
) {
  const editorState = useBenchState();
  const elementBounding = useElementBounding(container);
  const context: EditorContext<T> = {
    editor,
    component,
    container,
    size: computed(() => ({ width: elementBounding.width.value, height: elementBounding.height.value })),
    pos: computed(() => ({
      left: elementBounding.left.value,
      top: elementBounding.top.value,
    })),
    actions: computed(() => {
      const actions: EditorAction[] = [
        {
          groupId: "close",
          label: "Close",
          icon: XCircleIcon,
          action: () => editorState.closeEditor(editor.value),
        },
        {
          groupId: "close",
          label: "Close Others",
          icon: XCircleIcon,
          action: () =>
            editor.value.group?.editors.filter((e) => e != editor.value).forEach((e) => editorState.closeEditor(e)),
        },
        {
          groupId: "close",
          label: "Close All",
          icon: XCircleIcon,
          action: () => editorState.closeEditorGroup(editor.value.group as EditorGroup),
        },
      ];
      if (editor.value.effectiveWide) {
        actions.push({
          groupId: "view",
          label: "Narrow",
          icon: ArrowsPointingInIcon,
          action: () => (editor.value.appearance.wide = false),
        });
      } else {
        actions.push({
          groupId: "view",
          label: "Expand",
          icon: ArrowsPointingOutIcon,
          action: () => (editor.value.appearance.wide = true),
        });
      }
      if (editor.value.groupId == editorState.left.id) {
        actions.push({
          groupId: "move",
          label: "Move Right",
          icon: ArrowRightIcon,
          action: () => editorState.moveEditor(editor.value, editorState.right),
        });
        actions.push({
          groupId: "move",
          label: "Split Right",
          icon: ArrowRightIcon,
          action: () => editorState.moveEditor(editor.value, editorState.right, { copy: true }),
        });
      } else {
        actions.push({
          groupId: "move",
          label: "Move Left",
          icon: ArrowLeftIcon,
          action: () => editorState.moveEditor(editor.value, editorState.left),
        });
        actions.push({
          groupId: "move",
          label: "Split Left",
          icon: ArrowLeftIcon,
          action: () => editorState.moveEditor(editor.value, editorState.left, { copy: true }),
        });
      }
      return actions;
    }),
    actionGroups: computed(() => [
      { id: "close", label: "Close" },
      { id: "view", label: "View" },
      { id: "move", label: "Move" },
    ]),
  };
  provide(EDITOR_CONTEXT, context);
  return context;
}

export function useEditorContext<T extends Editor>(): EditorContext<T> {
  const context = inject<EditorContext<T>>(EDITOR_CONTEXT);
  if (context == null) {
    throw new Error("scroll context not provided");
  }
  return context;
}

// actions

export type Action<T> = {
  label: string;
  icon: any;
  action: (item: T) => void;
  active?: boolean;
  disabled?: boolean;
  keepOpen?: boolean;
  hideInline?: boolean;
  groupId?: string;
};

export type ActionGroup = {
  id: string;
  label?: string;
  icon?: any;
};

export type FileAction = Action<FileHeader>;
export type StatementAction = Action<StatementHeader>;
export type TypeAction = Action<Field>;
export type RecordAction = Action<BRecord>;
export type EditorAction = Action<Editor>;

// specific editors

export type NavElementType = "Statement" | "Field" | "Record";
export type NavElement = { id: Scalars["GlobalID"]; __typename?: NavElementType };

export abstract class NavigableEditor extends Editor {
  activeStatementId?: string;
  selectedElementType?: NavElementType;
  selectedElementIds: string[] = [];
  editing = false;

  focusElement(element: NavElement, retainEditing = false) {
    if (element.__typename != "Statement") {
      throw new Error(`focusElement only supports Statement elements, got ${element.__typename}`);
    }
    if (this.activeStatementId == element.id) return;
    console.debug(`focus element ${element.id}`);
    this.activeStatementId = element.id;
    this.editing = this.editing && retainEditing;
  }

  blurElement(element?: NavElement) {
    if (element == null || element.id == this.activeStatementId) {
      this.activeStatementId = undefined;
      this.editing = false;
      console.debug(`blur element ${element?.id}`);
    }
  }

  editElement(element: NavElement) {
    this.focusElement(element);
    this.editing = true;
    console.debug(`edit element ${element.id}`);
  }

  stopEditingElement(element?: NavElement) {
    if (element == null || element.id == this.activeStatementId) {
      this.editing = false;
    }
  }

  get hasSelection(): boolean {
    return (this.selectedElementIds?.length ?? 0) > 0;
  }

  get previousSelectedStatementId(): string | null {
    const selection = this.getSelection("Statement");
    return selection?.[selection.length - 2] ?? null;
  }

  get currentSelectedStatementId(): string | null {
    const selection = this.getSelection("Statement");
    return selection?.[selection.length - 1] ?? null;
  }

  getSelection(__typename?: NavElementType): string[] | undefined {
    if (this.selectedElementType != __typename) return undefined;
    return this.selectedElementIds;
  }

  isSelected(element: NavElement): boolean {
    return this.getSelection(element.__typename)?.includes(element.id) ?? false;
  }

  addToSelection(element: NavElement): void {
    if (element.__typename == null) throw new Error(`element ${element.id} has no __typename`);
    if (this.selectedElementType != element.__typename) {
      // reset selection
      this.selectedElementType = element.__typename;
      this.selectedElementIds = [];
    }
    const selection = this.getSelection(element.__typename) as string[];
    if (selection.find((e) => e == element.id)) return;
    console.debug("add to selection", this.path, element.id, selection.length);
    selection.push(element.id);
  }

  removeFromSelection(element: NavElement) {
    const selection = this.getSelection(element.__typename);
    if (selection == null) return;
    console.debug("remove from selection", this.path, element.id);
    this.selectedElementIds = selection.filter((id) => id != element.id);
  }

  clearSelection(): void {
    if (!this.hasSelection) return;
    console.debug("clear selection", this.path);
    this.selectedElementIds = [];
  }
}

export class FileEditor extends NavigableEditor {
  type = "file" as const;
  fileId: string;

  constructor(file: { id: string; name: string }) {
    super("file", file.id + "-" + randomHexString(), file.name, file.name, null);
    this.fileId = file.id;
  }

  resetId(): void {
    this.id = this.fileId + "-" + randomHexString();
  }

  updatePath(fileHeader: { id: string; name?: string | null }, module: ModuleIndex) {
    const file = module.filesById[fileHeader.id];
    if (file == null) return;
    this.name = fileHeader.name ?? file.name;
    this.path = file.name;
  }

  static parsePath(path: string, module: ModuleIndex): Editor | null {
    const matchingFile = Object.values(module.filesById).find((f) => prettifySlug(f.name) == path);
    if (matchingFile == null) return null;
    return new FileEditor(matchingFile);
  }
}

export class StatementEditor extends NavigableEditor {
  type = "statement" as const;
  statementId: string;

  constructor(statement: { id: string; name?: string | null }) {
    super("statement", statement.id + "-" + randomHexString(), statement.name ?? "", statement.name ?? "", null);
    this.statementId = statement.id;
  }

  resetId(): void {
    this.id = this.statementId + "-" + randomHexString();
  }

  updatePath(statementHeader: { id: string; name?: string | null }, module: ModuleIndex) {
    const statement = module.statementsById[statementHeader.id];
    const file = module.filesById[statement?.file.id ?? ""];
    if (statement == null || file == null) return;
    this.name = statementHeader.name ?? statement.name ?? "";
    this.path = `${file.name}:${statement.name ?? ""}`;
  }

  static parsePath(path: string, module: ModuleIndex): Editor | null {
    const [filePath, statementName] = path.split(":");
    const matchingFile = Object.values(module.filesById).find((f) => prettifySlug(f.name) == filePath);
    if (matchingFile == null) return null;
    const matchingStatement = module.statementsByFileId[matchingFile.id].find(
      (s) => prettifySlug(s.name ?? "") == statementName
    );
    if (matchingStatement == null) return null;
    return new StatementEditor(matchingStatement);
  }
}

export class LaunchEditor extends Editor {
  type = "launch" as const;
  statementId: string;
  statementType?: StatementType.Task | StatementType.Code;
  arguments: Record<string, any> = {};
  lastOutput?: Record<string, any> = {};
  lastExecutionTerminatedAt?: string;
  lastExecutionId?: string;

  constructor(statement: { id: string; name?: string | null; __typename?: string }) {
    super("launch", statement.id + "-" + randomHexString(), statement.name ?? "", statement.name ?? "");
    this.statementId = statement.id;
    if (statement.__typename == "Task") {
      this.statementType = StatementType.Task;
    } else if (statement.__typename == "Code") {
      this.statementType = StatementType.Code;
    }
  }

  resetId(): void {
    this.id = this.statementId + "-" + randomHexString();
  }

  updatePath(statementHeader: { id: string; name?: string | null }, module: ModuleIndex) {
    const statement = module.statementsById[statementHeader.id];
    const file = module.filesById[statement?.file?.id ?? ""];
    if (statement == null || file == null) return;
    this.name = statementHeader.name ?? statement.name ?? "";
    this.path = `${file.name}:${statement.name ?? ""}@${this.type}`;
  }

  static parsePath(path: string, module: ModuleIndex): Editor | null {
    const [filePath, statementName] = path.split(":");
    const matchingFile = Object.values(module.filesById).find((f) => prettifySlug(f.name) == filePath);
    if (matchingFile == null) return null;
    const matchingStatement = module.statementsByFileId[matchingFile.id].find(
      (s) => prettifySlug(s.name ?? "") == statementName
    );
    if (matchingStatement == null) return null;
    return new LaunchEditor(matchingStatement);
  }

  get hasWhiteBackground() {
    return false;
  }

  clear() {
    this.arguments = {};
  }
}

export const EDITOR_INSTANCE_TYPES: Record<EditorType, typeof Editor> = {
  file: FileEditor as any,
  statement: StatementEditor as any,
  launch: LaunchEditor as any, // don't care about constructor type
};

function instantiate(editorData: any, bench: ReturnType<typeof useBenchState>): Editor {
  const type = editorData.type;
  const EditorClass = EDITOR_INSTANCE_TYPES[type as EditorType];
  if (EditorClass == null) {
    throw new Error(`unknown editor type ${type}`);
  }
  if (!Reflect.setPrototypeOf(editorData, EditorClass.prototype)) {
    throw new Error(`failed to set prototype of editor ${editorData.id}`);
  }
  const editor = editorData as Editor;
  editor.onDeserialized(bench);
  return editor;
}
