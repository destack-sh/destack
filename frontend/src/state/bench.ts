/* eslint-disable @typescript-eslint/no-explicit-any */
import { graphql } from "@/gql";
import {
  StatementType,
  type Record as BRecord,
  type Field,
  type File,
  type Project,
  type ProjectVersion,
  type Scalars,
  type Statement,
  type SearchSort,
} from "@/gql/graphql";
import {
  CONTENT_MARGIN_X_NARROW,
  CONTENT_MARGIN_X_WIDE,
  CONTENT_WIDTH_NARROW,
  CONTENT_WIDTH_WIDE,
  useAppearanceState,
  type PanelAppearance,
  type Theme,
} from "@/state/appearance";
import type { ModuleIndex, NodeBase } from "@/state/module";
import type { LogsQuery, RunsQuery } from "@/state/session";
import { getUUIDFromGlobalID, randomHexString } from "@/utils/functools";
import {
  ArrowLeftIcon,
  ArrowRightIcon,
  ArrowsPointingInIcon,
  ArrowsPointingOutIcon,
  Bars4Icon as Bars4IconOutline,
  CodeBracketIcon as CodeBracketIconOutline,
  PlayIcon as PlayIconOutline,
  XCircleIcon,
  WindowIcon as WindowIconOutline,
} from "@heroicons/vue/24/outline";
import {
  CodeBracketIcon as CodeBracketIconSolid,
  PlayIcon as PlayIconSolid,
  Bars4Icon as Bars4IconSolid,
  WindowIcon as WindowIconSolid,
} from "@heroicons/vue/24/solid";
import { useApolloClient } from "@vue/apollo-composable";
import { useElementBounding } from "@vueuse/core";
import { defineStore } from "pinia";
import { computed, inject, onBeforeUnmount, provide, ref, watch, type Ref } from "vue";

export type ProjectHeader = Pick<Project, "id" | "name" | "slug" | "canWrite" | "createdAt" | "updatedAt">;
export type ProjectVersionHeader = Pick<
  ProjectVersion,
  "id" | "name" | "description" | "createdAt" | "committed" | "committedAt"
>;
export type FileHeader = Pick<File, "__typename" | "id" | "name" | "createdAt" | "updatedAt" | "deletedAt" | "parent">;
export type StatementHeader = Pick<
  Statement,
  "__typename" | "id" | "type" | "name" | "createdAt" | "updatedAt" | "deletedAt" | "orderKey" | "parent"
>;

export type ViewId = "explorer" | "search" | "history" | "issues" | "environment";

export type PanelType = "edit-file" | "edit-statement" | "launch-run" | "view-run" | "view-runs" | "view-logs";

const BENCH_STATE_VERSION = 4;

export function prettifySlug(path: string) {
  // replace non-URL friendly characters with dashes
  return path.replace(/[^a-zA-Z0-9-_./@:]/g, "-");
}

// note that panel state should be JSON serializable (except below)
const UNSERIALIZABLE_PANEL_PROPS = ["_bench", "_context"];
export abstract class Panel {
  type: PanelType;
  id: string;
  name: string;
  path: string;
  groupId: string | null = null; // id instead of PanelGroup to avoid circular dependency
  appearance: PanelAppearance = {};
  lastFocusedAt: string | null = null;
  lastActiveAt: string | null = null;
  // refs assigned on creation/component instantiation
  _bench: ReturnType<typeof useBenchState> | undefined = undefined;
  _context: any | undefined = undefined;

  constructor(type: PanelType, id: string, name: string, path: string, groupId: string | null = null) {
    this.name = name;
    this.type = type;
    this.id = id;
    this.path = path;
    this.groupId = groupId;
  }

  copy(): Panel {
    // serialize, deserialize and reset id
    const serialized = JSON.stringify(stripPanel(this));
    const copy = instantiate(JSON.parse(serialized), this.bench);
    copy.resetId();
    return copy;
  }

  onDeserialized(bench: ReturnType<typeof useBenchState>) {
    this._bench = bench;
  }

  onMounted(context: PanelContext<any>) {
    if (this._context != null) {
      throw new Error(`panel ${this.id} already has a context`);
    }
    this._context = context;
  }

  onUnmounted() {
    this._context = undefined;
  }
  abstract resetId(): void;

  static parsePath(path: string, module: ModuleIndex): Panel | null {
    throw new Error("not implemented"); // implement per panel type
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
  get focused() {
    return this._bench?.focusedPanelId == this.id;
  }

  get group(): PanelGroup | undefined {
    if (this.groupId == null) {
      return undefined;
    }
    return this.bench.groups.find((g) => g.id == this.groupId);
  }

  get bench(): ReturnType<typeof useBenchState> {
    if (this._bench == null) {
      throw new Error(`panel ${this.id} has no bench`);
    }
    return this._bench;
  }

  get context(): any {
    // TODO @Cleanup: this should be typed but TS throws up
    if (this._context == null) {
      throw new Error(`panel ${this.id} has no context`);
    }
    return this._context;
  }

  blur() {
    // noop by default
  }
}

export type PanelGroup = {
  id: string;
  name: string;
  panels: Panel[];
  activePanelId: string | null;
};

function makePanelGroup(id: string, name: string): PanelGroup {
  return {
    id,
    name,
    panels: [],
    activePanelId: null,
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
      // panels
      left: makePanelGroup("left", "Left"),
      right: makePanelGroup("right", "Right"),
      focusedPanelId: null as string | null,
      // appearance/settings (should be merged into appearance? but is bench specific...)
      debug: false,
      showPanelTabs: true,
      showBenchHeader: true,
      showViewSelection: true,
      showViewContent: false,
      zenMode: false,
    };
  },
  getters: {
    groups(state) {
      return [state.left, state.right];
    },
    group(): (id: string) => PanelGroup {
      return (id: string) => {
        const group = this.groups.find((g) => g.id == id);
        if (group == null) throw new Error(`panel group ${id} not found`);
        return group;
      };
    },
    panels(state) {
      return state.left.panels.concat(state.right.panels);
    },
    focusedPanel(): Panel | undefined {
      return this.panels.find((e) => e.id == this.focusedPanelId);
    },
    focusedFileId(): string | null {
      return this.focusedPanel?.type == "edit-file" ? (this.focusedPanel as EditFilePanel).fileId : null;
    },
    focusedFile(): EditFilePanel | undefined {
      return this.focusedPanel?.type == "edit-file" ? (this.focusedPanel as EditFilePanel) : undefined;
    },
    focusedStatementId(): string | null {
      return this.focusedFile?.activeStatementId ?? null;
    },
    focusedGroup(): PanelGroup | undefined {
      if (this.focusedPanel?.groupId == null) return undefined;
      return this.group(this.focusedPanel?.groupId);
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

    // panels

    _removePanelFromGroup(panel: Panel): void {
      if (panel.groupId == null) return;
      const group = this.group(panel.groupId);
      if (group == null) return;
      group.panels = group.panels.filter((e) => e != panel);
      if (group.activePanelId == panel.id) {
        // if active panel was removed, set first panel as active
        const nextToFocus = group.panels.sort((a, b) => ((a.lastActiveAt ?? "") > (b.lastActiveAt ?? "") ? -1 : 1))[0];
        if (nextToFocus != null) {
          group.activePanelId = nextToFocus.id;
          nextToFocus.lastActiveAt = new Date().toISOString();
        }
        group.activePanelId = nextToFocus?.id || null;
        // if panel was focused, focus new active panel
        if (panel.id == this.focusedPanelId) {
          this.focusedPanelId = group.activePanelId;
          if (nextToFocus != null) {
            nextToFocus.lastFocusedAt = new Date().toISOString();
          }
        }
      }
      panel.groupId = null;
    },

    openPanel(panel: Panel, group?: PanelGroup): Panel {
      // if group wasn't passed, just return the panel if it's already open
      if (group == null && panel.groupId != null) {
        return panel;
      }
      console.log(`open panel ${panel.path} in group ${group?.id}`);
      group = group || this.focusedGroup || this.left;
      // change panel group if different
      if (panel.groupId != group.id) {
        if (panel.groupId != null) {
          // remove from old group
          this._removePanelFromGroup(panel);
        }
        panel.groupId = group.id;
        group.panels.push(panel);
      }
      return panel;
    },

    closePanel(panel: Panel): void {
      console.log(`close panel ${panel.path}`);
      if (panel.groupId != null) {
        this._removePanelFromGroup(panel);
      }
    },

    closePanelGroup(group: PanelGroup): void {
      console.log(`close panel group ${group.id}`);
      group.panels.forEach((e) => this.closePanel(e));
    },

    movePanel(panel: Panel, group: PanelGroup, options?: { copy?: boolean }): void {
      const wasFocused = panel == this.focusedPanel;
      if (options?.copy) {
        panel = panel.copy();
      }
      this.openPanel(panel, group);
      if (wasFocused) {
        this.focusPanel(panel);
      }
    },

    openFile(file: NodeBase, options?: { group?: PanelGroup; create?: boolean; focus?: boolean }): Panel {
      let panel = this.panels.find((e) => e.type == "edit-file" && (e as EditFilePanel).fileId == file.id);
      if (!panel || options?.create) {
        console.log(`create new file panel for ${file.id} ${file.name}`);
        panel = new EditFilePanel(file);
        panel.onDeserialized(this);
      }
      this.openPanel(panel, options?.group);
      if (options?.focus) {
        this.focusPanel(panel);
      }
      return panel;
    },

    openStatement(
      statement: { id: string; name?: string | null },
      options?: { group?: PanelGroup; create?: boolean; focus?: boolean }
    ): Panel {
      let panel = this.panels.find(
        (e) => e.type == "edit-statement" && (e as EditStatementPanel).statementId == statement.id
      );
      if (!panel || options?.create) {
        console.log(`create new statement panel for ${statement.name}`);
        panel = new EditStatementPanel(statement);
        panel.appearance.wide = true; // default to wide
        panel.onDeserialized(this);
      }
      this.openPanel(panel, options?.group);
      if (options?.focus) {
        this.focusPanel(panel);
      }
      return panel;
    },

    openLaunch(
      statement: { id: string; name?: string | null },
      options?: { group?: PanelGroup; create?: boolean; focus?: boolean }
    ): Panel {
      let panel = this.panels.find((e) => e.type == "launch-run" && (e as LaunchRunPanel).statementId == statement.id);
      if (!panel || options?.create) {
        console.log(`create new launch panel for ${statement.name}`);
        panel = new LaunchRunPanel(statement);
        panel.onDeserialized(this);
      }
      this.openPanel(panel, options?.group);
      if (options?.focus) {
        this.focusPanel(panel);
      }
      return panel;
    },

    openRuns(query?: RunsQuery, options?: { group?: PanelGroup; create?: boolean; focus?: boolean }): Panel {
      let panel = this.panels.find((e) => e.type == "view-runs" && (e as ViewRunsPanel).query == query);
      if (!panel || options?.create) {
        console.log(`create new runs panel`);
        panel = new ViewRunsPanel(query);
        panel.onDeserialized(this);
      }
      this.openPanel(panel, options?.group);
      if (options?.focus) {
        this.focusPanel(panel);
      }
      return panel;
    },

    openRun(run: { id: string }, options?: { group?: PanelGroup; create?: boolean; focus?: boolean }): Panel {
      let panel = this.panels.find((e) => e.type == "view-run" && (e as ViewRunPanel).runId == run.id);
      if (!panel || options?.create) {
        console.log(`create new run panel for ${run.id}`);
        panel = new ViewRunPanel(run);
        panel.onDeserialized(this);
      }
      this.openPanel(panel, options?.group);
      if (options?.focus) {
        this.focusPanel(panel);
      }
      return panel;
    },

    nextGroup(group: PanelGroup): PanelGroup | undefined {
      const index = this.groups.indexOf(group);
      return this.groups[(index + 1) % this.groups.length];
    },

    focusGroup(group: PanelGroup): void {
      if (this.focusedGroup?.id == group.id) return;
      if (group.activePanelId == null) {
        throw new Error("group must have an active panel");
      }
      console.debug(`focus group ${group.id}`);
      this.focusedPanelId = group.activePanelId;
      if (this.focusedPanel != null) {
        this.focusedPanel.lastFocusedAt = new Date().toISOString();
      }
    },

    focusPanel(panel: Panel): void {
      this.focusedViewId = null;
      if (this.focusedPanel?.id == panel.id) return;
      console.debug(`focus panel ${panel.path} in group ${panel.groupId}`);
      if (!panel.groupId) {
        throw new Error("panel must be in a group: " + panel.path);
      }
      // blur all other panels
      this.panels.filter((e) => e.id != panel.id).forEach((e) => e.blur());
      this.focusedPanelId = panel.id;
      this.group(panel.groupId).activePanelId = panel.id;
      panel.lastFocusedAt = new Date().toISOString();
      panel.lastActiveAt = new Date().toISOString();
    },

    focusFile(file: NodeBase, group?: PanelGroup): Panel {
      const panel = this.openFile(file, { group });
      this.focusPanel(panel);
      return panel;
    },

    focusStatement(statement: NodeBase, group?: PanelGroup): Panel {
      const panel = this.openStatement(statement, { group });
      this.focusPanel(panel);
      return panel;
    },

    focusNode(node: NodeBase, group?: PanelGroup): Panel {
      if (node.__typename == "Statement") {
        return this.focusStatement(node, group);
      } else if (node.__typename == "File") {
        return this.focusFile(node, group);
      } else {
        throw new Error(`cannot focus node ${node.__typename}`);
      }
    },

    blur() {
      this.panels.forEach((e) => e.blur());
    },

    // settings

    setZenMode(zenMode: boolean) {
      const appearance = useAppearanceState();
      this.zenMode = zenMode;
      this.showViewContent = !zenMode;
      this.showBenchHeader = !zenMode;
      appearance.fullscreen = zenMode;
    },

    // migration

    _doMigrateTo(versionId: string, refMappings: Record<string, string>): void {
      // migrate by serializing state and replacing refs
      let stateJson = benchStateToJson(this);
      // TODO @Performance: replace panel refs on migration in a single pass
      for (const [sourceId, targetId] of Object.entries(refMappings)) {
        // replace all matches of ref.source with ref.target
        // (need to use regex to replace *all* matches)
        const re = new RegExp(`"${sourceId}"`, "g");
        stateJson = stateJson.replace(re, `"${targetId}"`);
      }
      this.$reset();
      benchInitFromJson(this, stateJson);

      // remove panels with refs we don't have anymore
      // note that this also closes any module-external refs
      // I tried to fix this by only removing refs we _used_ tohave (checking for original ref.target)
      // but that doesn't work for refs that were just created in first source version.
      const targetRefs = Object.values(refMappings);
      for (const panel of this.panels) {
        let panelRef = null;
        if (panel.type == "edit-file") {
          panelRef = (panel as EditFilePanel).fileId;
        }
        if (panelRef != null && !targetRefs.includes(panelRef)) {
          console.debug(`close outdated panel ${panel.path} (${panel.id} pointed to ${panelRef})`);
          this.closePanel(panel);
        }
      }
      this.projectVersionId = versionId;
    },
  },
});

// persistence

function stripPanel(panel: Panel) {
  const stripped = { ...panel };
  for (const prop of UNSERIALIZABLE_PANEL_PROPS) {
    delete (stripped as any)[prop];
  }
  return stripped;
}

function benchStateToJson(bench: ReturnType<typeof useBenchState>): string {
  // clean up panel state for serialization
  const state = {
    ...bench.$state,
    left: {
      ...bench.$state.left,
      panels: bench.$state.left.panels.map((e) => stripPanel(e)),
    },
    right: {
      ...bench.$state.right,
      panels: bench.$state.right.panels.map((e) => stripPanel(e)),
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
  // instantiate panels
  for (const group of bench.groups) {
    group.panels = group.panels.map((e) => instantiate(e, bench));
  }
}

export function useBenchPersistence(minIntervalMs = 1000) {
  const bench = useBenchState();

  function save(projectId?: string) {
    if (bench.projectId == null) return;
    if (projectId != null && bench.projectId != projectId) {
      throw new Error(`cannot save panel state for project ${projectId} (current project is ${bench.projectId})`);
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
        console.warn(`unable to restore bench state for project ${projectId}`, e);
      }
    }
    return false;
  }

  // save every interval
  const interval = setInterval(save, minIntervalMs);
  onBeforeUnmount(() => {
    save();
    clearInterval(interval);
  });

  // and watch for any changes
  watch(
    () => bench.$state,
    () => save(),
    { deep: true }
  );

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

export type PanelContext<T extends Panel> = {
  panel: Ref<T>;
  component: Ref<any>;
  container: Ref<HTMLElement | null>;
  size: Ref<{ width: number; height: number }>;
  pos: Ref<{ left: number; top: number }>;
  scroll: Ref<{ x: number; y: number }>;
  actions: Ref<PanelAction[]>;
  actionGroups?: Ref<ActionGroup[]>;
};

export const PANEL_CONTEXT = "__panel__";

export function providePanelContext<T extends Panel>(
  panel: Ref<T>,
  component: Ref<any>,
  container: Ref<HTMLElement | null>,
  scroll: { x: Ref<number>; y: Ref<number> }
) {
  const bench = useBenchState();
  const elementBounding = useElementBounding(container);
  const context: PanelContext<T> = {
    panel: panel,
    component,
    container,
    size: computed(() => ({ width: elementBounding.width.value, height: elementBounding.height.value })),
    pos: computed(() => ({
      left: elementBounding.left.value,
      top: elementBounding.top.value,
    })),
    scroll: computed(() => ({ x: scroll.x.value, y: scroll.y.value })),
    actions: computed(() => {
      const actions: PanelAction[] = [];

      // open other panels in this group if there are any
      if (!bench.showPanelTabs) {
        panel.value.group?.panels.forEach((e) => {
          if (e.id == panel.value.id) return;
          actions.push({
            groupId: "jump",
            label: e.name.length > 0 ? e.name : "(Unnamed)",
            icon: PANEL_ICONS_OUTLINE[e.type],
            action: () => bench.focusPanel(e),
          });
        });
      }

      // close
      const closeActions = [
        {
          groupId: "close",
          label: "Close",
          icon: XCircleIcon,
          action: () => bench.closePanel(panel.value),
        },
        {
          groupId: "close",
          label: "Close Others",
          icon: XCircleIcon,
          action: () => panel.value.group?.panels.filter((e) => e != panel.value).forEach((e) => bench.closePanel(e)),
        },
        {
          groupId: "close",
          label: "Close All",
          icon: XCircleIcon,
          action: () => bench.closePanelGroup(panel.value.group as PanelGroup),
        },
      ];
      actions.push(...closeActions);

      // view
      if (panel.value.effectiveWide) {
        actions.push({
          groupId: "view",
          label: "Narrow",
          icon: ArrowsPointingInIcon,
          action: () => (panel.value.appearance.wide = false),
        });
      } else {
        actions.push({
          groupId: "view",
          label: "Expand",
          icon: ArrowsPointingOutIcon,
          action: () => (panel.value.appearance.wide = true),
        });
      }

      // move
      // should clean this up
      if (panel.value.groupId == bench.left.id) {
        actions.push({
          groupId: "move",
          label: "Move Right",
          icon: ArrowRightIcon,
          action: () => bench.movePanel(panel.value, bench.right),
        });
        actions.push({
          groupId: "move",
          label: "Split Right",
          icon: ArrowRightIcon,
          action: () => bench.movePanel(panel.value, bench.right, { copy: true }),
        });
      } else {
        actions.push({
          groupId: "move",
          label: "Move Left",
          icon: ArrowLeftIcon,
          action: () => bench.movePanel(panel.value, bench.left),
        });
        actions.push({
          groupId: "move",
          label: "Split Left",
          icon: ArrowLeftIcon,
          action: () => bench.movePanel(panel.value, bench.left, { copy: true }),
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
  provide(PANEL_CONTEXT, context);
  return context;
}

export function usePanelContext<T extends Panel>(): PanelContext<T> {
  const context = inject<PanelContext<T>>(PANEL_CONTEXT);
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
export type PanelAction = Action<Panel>;

// specific panels

export type NavElementType = "Statement" | "Field" | "Record";
export type NavElement = { id: Scalars["GlobalID"]; __typename?: NavElementType };

export abstract class NavigablePanel extends Panel {
  activeStatementId?: string;
  selectedElementType?: NavElementType;
  selectedElementIds: string[] = [];
  elementProperties: Record<string, any> = {};
  editing = false;

  onDeserialized(bench: ReturnType<typeof useBenchState>) {
    super.onDeserialized(bench);
    this.elementProperties = this.elementProperties || {};
  }

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

export class EditFilePanel extends NavigablePanel {
  type = "edit-file" as const;
  fileId: string;
  foldedStatementContentIds?: string[] = [];
  foldedStatementTreeIds?: string[] = [];

  constructor(file: NodeBase) {
    super("edit-file", file.id + "-" + randomHexString(), file.name ?? "(Unnamed)", file.name ?? "(Unnamed)", null);
    this.fileId = file.id;
  }

  resetId(): void {
    this.id = this.fileId + "-" + randomHexString();
  }

  isStatementContentFolded(statement: { id: string }): boolean {
    return this.foldedStatementContentIds?.includes(statement.id) ?? false;
  }

  toggleStatementContentFolded(statement: { id: string }): void {
    if (this.isStatementContentFolded(statement)) {
      this.foldedStatementContentIds = this.foldedStatementContentIds?.filter((id) => id != statement.id);
    } else {
      this.foldedStatementContentIds = this.foldedStatementContentIds ?? [];
      this.foldedStatementContentIds.push(statement.id);
    }
  }

  setStatementContentsFolded(statements: { id: string }[], folded: boolean): void {
    if (folded) {
      this.foldedStatementContentIds = [...(this.foldedStatementContentIds ?? []), ...statements.map((s) => s.id)];
    } else {
      this.foldedStatementContentIds = this.foldedStatementContentIds?.filter(
        (id) => !statements.find((s) => s.id == id)
      );
    }
  }

  updatePath(fileHeader: { id: string; name?: string | null }, module: ModuleIndex) {
    const file = module.filesById[fileHeader.id];
    if (file == null) return;
    this.name = fileHeader.name ?? file.name;
    this.path = file.name;
  }

  static parsePath(path: string, module: ModuleIndex): Panel | null {
    const matchingFile = Object.values(module.filesById).find((f) => prettifySlug(f.name) == path);
    if (matchingFile == null) return null;
    return new EditFilePanel(matchingFile as NodeBase);
  }
}

export class EditStatementPanel extends NavigablePanel {
  type = "edit-statement" as const;
  statementId: string;

  constructor(statement: { id: string; name?: string | null }) {
    super("edit-statement", statement.id + "-" + randomHexString(), statement.name ?? "", statement.name ?? "", null);
    this.statementId = statement.id;
  }

  get contentMarginX() {
    // always full width
    return 0;
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

  static parsePath(path: string, module: ModuleIndex): Panel | null {
    const [filePath, statementName] = path.split(":");
    const matchingFile = Object.values(module.filesById).find((f) => prettifySlug(f.name) == filePath);
    if (matchingFile == null) return null;
    const matchingStatement = module.statementsByFileId[matchingFile.id].find(
      (s) => prettifySlug(s.name ?? "") == statementName
    );
    if (matchingStatement == null) return null;
    return new EditStatementPanel(matchingStatement);
  }
}

export class LaunchRunPanel extends Panel {
  type = "launch-run" as const;
  statementId: string;
  statementType?: StatementType.Task | StatementType.Code;
  inputs: Record<string, any> = {};
  lastOutput?: Record<string, any> = {};
  lastError?: Record<string, any> = {};
  lastRunTerminatedAt?: string;
  lastRunId?: string;
  lastSessionId?: string;

  constructor(statement: { id: string; name?: string | null; __typename?: string }) {
    super("launch-run", statement.id + "-" + randomHexString(), statement.name ?? "", statement.name ?? "");
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

  static parsePath(path: string, module: ModuleIndex): Panel | null {
    const [filePath, statementName] = path.split(":");
    const matchingFile = Object.values(module.filesById).find((f) => prettifySlug(f.name) == filePath);
    if (matchingFile == null) return null;
    const matchingStatement = module.statementsByFileId[matchingFile.id].find(
      (s) => prettifySlug(s.name ?? "") == statementName
    );
    if (matchingStatement == null) return null;
    return new LaunchRunPanel(matchingStatement);
  }

  get hasWhiteBackground() {
    return false;
  }

  clear() {
    this.inputs = {};
  }
}

export class ViewRunsPanel extends Panel {
  type = "view-runs" as const;
  limit: number;
  query?: RunsQuery;
  sort?: SearchSort;

  constructor(query?: RunsQuery, sort?: SearchSort) {
    super("view-runs", "runs-" + randomHexString(), "Runs", "runs");
    this.limit = 50;
    this.query = query;
    this.sort = sort;
  }

  resetId(): void {
    this.id = "runs-" + randomHexString();
  }
  static parsePath(path: string, module: ModuleIndex): Panel | null {
    return null;
  }

  get hasWhiteBackground() {
    return false;
  }
}

export class ViewRunPanel extends Panel {
  type = "view-run" as const;
  runId: string;

  constructor(run: { id: string }) {
    super(
      "view-run",
      run.id + "-" + randomHexString(),
      "Run #" + getUUIDFromGlobalID(run.id).slice(-7, -1),
      "run:" + run.id
    );
    this.runId = run.id;
  }

  resetId(): void {
    this.id = this.runId + "-" + randomHexString();
  }

  static parsePath(path: string, module: ModuleIndex): Panel | null {
    return null;
  }

  get hasWhiteBackground() {
    return false;
  }
}

export class ViewLogsPanel extends Panel {
  type = "view-logs" as const;
  query?: LogsQuery;
  sort?: SearchSort;

  constructor() {
    super("view-logs", "logs-" + randomHexString(), "Logs", "Logs");
  }

  resetId(): void {
    this.id = "logs-" + randomHexString();
  }

  static parsePath(path: string, module: ModuleIndex): Panel | null {
    return null;
  }

  get hasWhiteBackground() {
    return false;
  }
}

export const PANEL_INSTANCE_TYPES: Record<PanelType, typeof Panel> = {
  "edit-file": EditFilePanel as any,
  "edit-statement": EditStatementPanel as any,
  "launch-run": LaunchRunPanel as any, // don't care about constructor type
  "view-runs": ViewRunsPanel as any,
  "view-run": ViewRunPanel as any,
  "view-logs": ViewLogsPanel as any,
};

export const PANEL_ICONS_OUTLINE: Record<PanelType, any> = {
  "edit-file": CodeBracketIconOutline,
  "edit-statement": CodeBracketIconOutline,
  "launch-run": WindowIconOutline,
  "view-runs": PlayIconOutline,
  "view-run": PlayIconOutline,
  "view-logs": Bars4IconOutline,
};

export const PANEL_ICONS_SOLID: Record<PanelType, any> = {
  "edit-file": CodeBracketIconSolid,
  "edit-statement": CodeBracketIconSolid,
  "launch-run": WindowIconSolid,
  "view-runs": PlayIconSolid,
  "view-run": PlayIconSolid,
  "view-logs": Bars4IconSolid,
};

function instantiate(panelData: any, bench: ReturnType<typeof useBenchState>): Panel {
  const type = panelData.type;
  const PanelClass = PANEL_INSTANCE_TYPES[type as PanelType];
  if (PanelClass == null) {
    throw new Error(`unknown panel type ${type}`);
  }
  if (!Reflect.setPrototypeOf(panelData, PanelClass.prototype)) {
    throw new Error(`failed to set prototype of panel ${panelData.id}`);
  }
  const panel = panelData as Panel;
  panel.onDeserialized(bench);
  return panel;
}

export function useElementPanelSettings<T>(element: Ref<{ id: string }>, defaultValue: T) {
  const panel = usePanelContext().panel;
  if (panel.value == null) {
    throw new Error("panel not set");
  }

  const proxy = new Proxy(
    {},
    {
      get: function (_: any, p: PropertyKey): T[keyof T] {
        const key = p as keyof T;
        const e = panel.value as NavigablePanel;
        const settings = e.elementProperties[element.value.id] || {};
        return settings[key] ?? defaultValue[key];
      },
      set: function (_: any, p: PropertyKey, value: any): boolean {
        const key = p as keyof T;
        const e = panel.value as NavigablePanel;
        const settings = e.elementProperties[element.value.id] || {};
        settings[key] = value as T[keyof T];
        e.elementProperties[element.value.id] = settings;
        return true;
      },
    }
  );
  return proxy as T;
}
