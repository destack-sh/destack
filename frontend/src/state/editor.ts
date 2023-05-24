import { graphql } from "@/gql";
import type { DatasetRecord, File, Project, ProjectVersion, Scalars, SimpleType, Statement } from "@/gql/graphql";
import { useAppearanceState, type Theme } from "@/state/appearance";
import { useNotifications } from "@/state/notifications";
import { ArrowLeftIcon, ArrowRightIcon } from "@heroicons/vue/24/outline";
import { useLazyQuery } from "@vue/apollo-composable";
import { useElementBounding } from "@vueuse/core";
import { defineStore } from "pinia";
import { computed, inject, onBeforeUnmount, provide, ref, watch, type Ref } from "vue";

export type ProjectHeader = Pick<Project, "id" | "name" | "slug" | "canWrite" | "createdAt" | "updatedAt">;
export type ProjectVersionHeader = Pick<
  ProjectVersion,
  "id" | "name" | "description" | "createdAt" | "committed" | "committedAt"
>;
export type FileHeader = Pick<
  File,
  "__typename" | "id" | "name" | "path" | "createdAt" | "updatedAt" | "deletedAt" | "directory" | "generated"
>;
export type StatementHeader = Pick<
  Statement,
  | "__typename"
  | "id"
  | "modifier"
  | "type"
  | "symbolType"
  | "name"
  | "createdAt"
  | "updatedAt"
  | "deletedAt"
  | "orderKey"
  | "generated"
  | "commented"
  | "parent"
  | "reference"
>;

export type ViewId = "explorer" | "search" | "history" | "issues" | "comments" | "environment" | "instruction";

export type EditorType = "file";

// note that editor state should be JSON serializable
export abstract class Editor {
  type: EditorType;
  id: string;
  path: string;
  groupId: string | null; // id instead of EditorGroup to avoid circular dependency
  editing = false;

  constructor(type: EditorType, id: string, path: string, groupId: string | null) {
    this.type = type;
    this.id = id;
    this.path = path;
    this.groupId = groupId;
  }

  blur() {
    this.editing = false;
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
      // bench state
      currentProjectId: null as string | null,
      currentProjectVersionId: null as string | null,
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
      this.currentProjectId = projectId;
      this.currentProjectVersionId = versionId;
    },

    setActiveView(viewId: ViewId): void {
      this.activeViewId = viewId;
    },

    openActiveView(viewId: ViewId): void {
      this.activeViewId = viewId;
      this.showViewContent = true;
    },

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

    moveEditor(editor: Editor, group: EditorGroup): void {
      const wasFocused = editor == this.focusedEditor;
      this.openEditor(editor, group);
      if (wasFocused) {
        this.focusEditor(editor);
      }
    },

    openFile(file: { id: string; path: string }, options?: { group?: EditorGroup; create?: boolean }): Editor {
      let editor = this.editors.find((e) => e.type == "file" && (e as FileEditor).fileId == file.id);
      if (!editor || options?.create) {
        console.log(`create new file editor for ${file.id} ${file.path}`);
        editor = new FileEditor(file);
      }
      return this.openEditor(editor, options?.group);
    },

    focusView(viewId: ViewId): void {
      if (viewId == this.focusedViewId) return;
      this.focusedViewId = viewId;
      this.openActiveView(viewId);
      this.blur();
      console.log(`focus view ${viewId}`);
    },

    blurView(viewId?: ViewId): void {
      if (viewId != null && viewId != this.focusedViewId) return;
      this.focusedViewId = null;
      console.log(`blur view ${viewId}`);
    },

    focusEditor(editor: Editor): void {
      this.focusedViewId = null;
      if (this.focusedEditor?.id == editor.id) return;

      console.debug(`focus editor ${editor.path} in group ${editor.groupId}`);
      if (!editor.groupId) {
        throw new Error("editor must be in a group: " + editor.path);
      }
      this.focusedEditorId = editor.id;
      this.group(editor.groupId).activeEditorId = editor.id;
    },

    focusFile(file: FileHeader, group?: EditorGroup): Editor {
      const editor = this.openFile(file, { group });
      this.focusEditor(editor);
      return editor;
    },

    blur() {
      this.editors.forEach((e) => e.blur());
    },

    setZenMode(zenMode: boolean) {
      const appearance = useAppearanceState();
      this.zenMode = zenMode;
      this.showViewContent = !zenMode;
      this.showGlobalHeader = !zenMode;
      appearance.fullscreen = zenMode;
    },

    async _doMigrateTo(versionId: string, refMappings: Record<string, string>): Promise<void> {
      // migrate by serializing state and replacing refs
      let stateJson = JSON.stringify(this.$state);
      for (const [sourceId, targetId] of Object.entries(refMappings)) {
        // replace all matches of ref.source with ref.target
        // (need to use regex to replace *all* matches)
        const re = new RegExp(`"${sourceId}"`, "g");
        stateJson = stateJson.replace(re, `"${targetId}"`);
      }
      this.$reset();
      this.$patch(JSON.parse(stateJson));

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
      this.currentProjectVersionId = versionId;
    },
  },
});

// persistence

export function useBenchPersistence(intervalMs = 1000) {
  const bench = useBenchState();

  const save = () => {
    // save editor state by project id
    if (bench.currentProjectId == null) return;
    localStorage.setItem(`editor-state-${bench.currentProjectId}`, JSON.stringify(bench.$state));
  };

  const load = () => {
    // load editor state by project id
    if (bench.currentProjectId == null) return;
    const state = localStorage.getItem(`editor-state-${bench.currentProjectId}`);
    if (state) {
      try {
        bench.$patch(JSON.parse(state));
        // instantiate editors
        for (const group of bench.groups) {
          group.editors = group.editors.map(instantiate);
        }
        console.log(`restored editor state for project ${bench.currentProjectId}`);
      } catch (e) {
        console.error(`failed to restore editor state for project ${bench.currentProjectId}`);
      }
    }
  };

  // save every interval
  const interval = setInterval(save, intervalMs);
  onBeforeUnmount(() => clearInterval(interval));

  return { save, load };
}

// migration

export function useBenchMigrations() {
  const migratingTo: Ref<string | null> = ref(null);
  const notifications = useNotifications();
  const bench = useBenchState();

  // TODO @Cleanup: project ref migration from vx to vy should be a server-side API endpoint
  const {
    load: getProjectMigrationRefs,
    loading: migrationLoading,
    error: migrationError,
    result: migrationRefs,
  } = useLazyQuery(
    graphql(/* GraphQL */ `
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
              type
              sourceId
              sourceVersionId
              targetId
              targetVersionId
            }
          }
        }
      }
    `)
  );

  // perform the migration once we've gotten the refs
  watch(
    () => [migrationRefs, migrationLoading, migrationError],
    async () => {
      if (migratingTo.value == null) return;
      if (migrationLoading.value) return;

      if (migrationError.value != null) {
        // fail migration
        bench.currentProjectVersionId = migratingTo.value;
        console.error("unable to migrate, error getting intermediate refs", migrationError.value);
        notifications.show({
          kind: "warning",
          type: "editorMigration.failed",
          message: "Migrating editor failed",
          description: "Editor could not be migrated (local only).",
        });
        migratingTo.value = null;
      } else if (migrationRefs.value != null) {
        const migrationMappings = migrationRefs.value.project.migrationMappings;
        const refMappings: Record<string, string> = {};
        for (const mapping of migrationMappings.refMappings) {
          refMappings[mapping.sourceId] = mapping.targetId;
        }
        await bench._doMigrateTo(migratingTo.value, refMappings);
        console.log(
          `migrated editor from version ${migrationMappings?.sourceVersion.tag} to ${migrationMappings?.targetVersion.tag}`
        );
        migratingTo.value = null;
      }
    },
    { deep: true }
  );

  function migrateTo(projectId: string, targetVersionId: string, sourceVersionId: string) {
    if (migratingTo.value != null) {
      if (migratingTo.value == targetVersionId) {
        // nothing to do
        return;
      } else {
        throw new Error(`already migrating to another version: ${migratingTo.value} (not ${targetVersionId})`);
      }
    }
    migratingTo.value = targetVersionId;
    console.log(
      `migrate editor state for project ${projectId} to version ${targetVersionId} (from version ${sourceVersionId}))`
    );
    // get all ref mappings
    getProjectMigrationRefs(undefined, {
      projectId: projectId,
      sourceVersionId: sourceVersionId,
      targetVersionId: targetVersionId,
    });
  }

  return {
    migrating: computed(() => migratingTo.value != null),
    migrateTo,
  };
}

// context

export type EditorContext<T extends Editor> = {
  editor: Ref<T>;
  container: Ref<HTMLElement | null>;
  size: Ref<{ width: number; height: number }>;
  pos: Ref<{ left: number; top: number }>;
  actions: Ref<EditorAction[]>;
};

export const EDITOR_CONTEXT = "__editor__";

export function provideEditorContext<T extends Editor>(editor: Ref<T>, container: Ref<HTMLElement | null>) {
  const editorState = useBenchState();
  const elementBounding = useElementBounding(container);
  const context: EditorContext<T> = {
    editor,
    container,
    size: computed(() => ({ width: elementBounding.width.value, height: elementBounding.height.value })),
    pos: computed(() => ({
      left: elementBounding.left.value,
      top: elementBounding.top.value,
    })),
    actions: computed(() => {
      const actions: EditorAction[] = [];
      if (editor.value.groupId == editorState.left.id) {
        actions.push({
          label: "Move to right",
          icon: ArrowRightIcon,
          action: () => editorState.moveEditor(editor.value, editorState.right),
        });
      } else {
        actions.push({
          label: "Move to left",
          icon: ArrowLeftIcon,
          action: () => editorState.moveEditor(editor.value, editorState.left),
        });
      }
      return actions;
    }),
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
};

export type FileAction = Action<FileHeader>;
export type StatementAction = Action<StatementHeader>;
export type TypeAction = Action<SimpleType>;
export type RecordAction = Action<DatasetRecord>;
export type EditorAction = Action<Editor>;

// specific editors

export type FileElementType = "Statement" | "SimpleTypeNode" | "DatasetRecord";
export type FileElement = { id: Scalars["GlobalID"]; __typename?: FileElementType };

export class FileEditor extends Editor {
  type = "file" as const;
  fileId: string;
  activeStatementId?: string;
  selectedElementType?: FileElementType;
  selectedElementIds: string[] = [];

  constructor(file: { id: string; path: string }) {
    super("file", file.id + "-" + Math.random().toString(16).substring(2, 8), file.path, null);
    this.fileId = file.id;
  }

  focusElement(element: FileElement, retainEditing = false) {
    if (element.__typename != "Statement") {
      throw new Error(`focusElement only supports Statement elements, got ${element.__typename}`);
    }
    if (this.activeStatementId == element.id) return;
    console.debug(`focus element ${element.id}`);
    this.activeStatementId = element.id;
    this.editing = this.editing && retainEditing;
  }

  blurElement(element?: FileElement) {
    if (element == null || element.id == this.activeStatementId) {
      this.activeStatementId = undefined;
      console.log(`blur element ${element?.id}`);
    }
  }

  editElement(element: FileElement) {
    this.focusElement(element);
    this.editing = true;
  }

  stopEditingElement(element?: FileElement) {
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

  getSelection(__typename?: FileElementType): string[] | undefined {
    if (this.selectedElementType != __typename) return undefined;
    return this.selectedElementIds;
  }

  isSelected(element: FileElement): boolean {
    return this.getSelection(element.__typename)?.includes(element.id) ?? false;
  }

  addToSelection(element: FileElement): void {
    const selection = this.getSelection(element.__typename);
    if (selection == null) return;
    if (selection.find((e) => e == element.id)) return;
    console.debug("add to selection", this.path, element.id, selection.length);
    selection.push(element.id);
  }

  removeFromSelection(element: FileElement) {
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

const EDITOR_INSTANCES: Record<EditorType, any> = {
  file: FileEditor,
};

function instantiate(editorData: any): Editor {
  const type = editorData.type;
  const EditorClass = EDITOR_INSTANCES[type as EditorType];
  if (EditorClass == null) {
    throw new Error(`unknown editor type ${type}`);
  }
  if (!Reflect.setPrototypeOf(editorData, EditorClass.prototype)) {
    throw new Error(`failed to set prototype of editor ${editorData.id}`);
  }
  return editorData;
}
