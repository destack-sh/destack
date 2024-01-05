/* eslint-disable @typescript-eslint/no-explicit-any */
import {
  CONTENT_MARGIN_X_NARROW,
  CONTENT_MARGIN_X_WIDE,
  CONTENT_WIDTH_NARROW,
  CONTENT_WIDTH_WIDE,
  useAppearanceState,
  type PanelAppearance,
  type Theme,
} from "@/state/appearance";
import { getNodeIdFromCkMaybe, type ModuleIndex, type NodeBase } from "@/state/module";
import { SessionAccessLevel, type LogsQuery, type RunsQuery } from "@/state/session";
import { getUUIDFromGlobalID, randomHexString, toGlobalId } from "@/utils/functools";
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
  ArrowUturnLeftIcon,
  CommandLineIcon as CommandLineIconOutline,
  CircleStackIcon as CircleStackIconOutline,
} from "@heroicons/vue/24/outline";
import {
  CodeBracketIcon as CodeBracketIconSolid,
  PlayIcon as PlayIconSolid,
  Bars4Icon as Bars4IconSolid,
  WindowIcon as WindowIconSolid,
  CommandLineIcon as CommandLineIconSolid,
  CircleStackIcon as CircleStackIconSolid,
} from "@heroicons/vue/24/solid";
import { useElementBounding } from "@vueuse/core";
import { defineStore } from "pinia";
import { computed, inject, onBeforeUnmount, provide, watch, type Ref, nextTick, toRef } from "vue";
import { validate as isValidUUID } from "uuid";
import type EditFilePanelVue from "@/components/panels/EditFilePanel.vue";
import { ModuleAccessLevel, dashifyUuid } from "@/state/auth";
import { useNotifications } from "@/state/notifications";

export const PROJECT_ACCESS_LEVELS = [
  ModuleAccessLevel.Zero,
  ModuleAccessLevel.Read,
  ModuleAccessLevel.Use,
  ModuleAccessLevel.Edit,
  ModuleAccessLevel.Manage,
  ModuleAccessLevel.Admin,
];

export const PROJECT_ACCESS_LEVEL_NAME: Record<ModuleAccessLevel, string> = {
  [ModuleAccessLevel.Zero]: "None",
  [ModuleAccessLevel.Read]: "Read",
  [ModuleAccessLevel.Use]: "Use",
  [ModuleAccessLevel.Edit]: "Edit",
  [ModuleAccessLevel.Manage]: "Manage",
  [ModuleAccessLevel.Admin]: "Admin",
};

export function projectAccessGt(a: ModuleAccessLevel, b: ModuleAccessLevel): boolean {
  return PROJECT_ACCESS_LEVELS.indexOf(a) > PROJECT_ACCESS_LEVELS.indexOf(b);
}

export function projectAccessGte(a: ModuleAccessLevel, b: ModuleAccessLevel): boolean {
  return PROJECT_ACCESS_LEVELS.indexOf(a) >= PROJECT_ACCESS_LEVELS.indexOf(b);
}

export function projectAccessLt(a: ModuleAccessLevel, b: ModuleAccessLevel): boolean {
  return PROJECT_ACCESS_LEVELS.indexOf(a) < PROJECT_ACCESS_LEVELS.indexOf(b);
}

export type ViewId = "explorer" | "search" | "history" | "issues" | "environment" | "tests" | "comments";

export type PanelType =
  | "edit-file"
  | "edit-database"
  | "launch-run"
  | "view-run"
  | "view-runs"
  | "view-logs"
  | "terminal";

const BENCH_STATE_VERSION = 10;

export function prettifySlug(path: string) {
  // replace non-URL friendly characters with dashes
  return path.replace(/[^a-zA-Z0-9-_./@:]/g, "-");
}

// note that panel state should be JSON serializable (except below)
const UNSERIALIZABLE_PANEL_PROPS = ["_bench", "_context", "_panel"];
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
  _context: any | PanelContext<any> = undefined;

  constructor(type: PanelType, id: string, name: string, path: string, groupId: string | null = null) {
    this.name = name;
    this.type = type;
    this.id = id;
    this.path = path;
    this.groupId = groupId;
  }

  copy(options?: { resetId?: boolean }): Panel {
    const serialized = JSON.stringify(stripPanel(this));
    const copy = instantiate(JSON.parse(serialized), this.bench);
    if (options?.resetId) copy.resetId();
    return copy;
  }

  onInstantiated(bench: ReturnType<typeof useBenchState>) {
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

  get hasScrollY() {
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

  get component(): InstanceType<any> {
    return this.context.component;
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

export const RECENTLY_CLOSED_PANELS_LIMIT = 128;

export type PanelOpenOptions = {
  group?: PanelGroup;
  opposite?: boolean;
  create?: boolean;
  focus?: boolean;
};

export const useBenchState = defineStore("bench", {
  state: () => {
    return {
      version: BENCH_STATE_VERSION,
      // bench
      projectId: null as string | null,
      projectVersionId: null as string | null,
      projectHeadId: null as string | null,
      accessLevel: null as ModuleAccessLevel | null,
      readonly: false,
      // views
      activeViewId: "explorer" as ViewId,
      focusedViewId: null as ViewId | null,
      // panels
      left: makePanelGroup("left", "Left"),
      right: makePanelGroup("right", "Right"),
      recentlyClosedPanels: [] as Panel[],
      focusedPanelId: null as string | null,
      // appearance/settings (should be merged into appearance? but is bench specific...)
      debug: false,
      keepEmptyPanelGroups: false,
      showPanelTabs: true,
      showPanelExplorer: false,
      showBenchHeader: true,
      showViewSelection: true,
      showViewContent: true,
      zenMode: false,
    };
  },
  getters: {
    // versioning
    isAtHead(): boolean {
      return this.projectVersionId == this.projectHeadId;
    },
    // access
    canRead(): boolean {
      return projectAccessGte(this.accessLevel ?? ModuleAccessLevel.Zero, ModuleAccessLevel.Read);
    },
    canUse(): boolean {
      return projectAccessGte(this.accessLevel ?? ModuleAccessLevel.Zero, ModuleAccessLevel.Use);
    },
    canEdit(): boolean {
      return projectAccessGt(this.accessLevel ?? ModuleAccessLevel.Zero, ModuleAccessLevel.Edit);
    },
    canManage(): boolean {
      return projectAccessGte(this.accessLevel ?? ModuleAccessLevel.Zero, ModuleAccessLevel.Manage);
    },
    // bench
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
    focusedFileCk(): string | null {
      return this.focusedPanel?.type == "edit-file" ? (this.focusedPanel as EditFilePanel).fileCk : null;
    },
    focusedFileId(): string | null {
      return getNodeIdFromCkMaybe(this.projectVersionId, this.focusedFileCk, "File");
    },
    focusedFile(): EditFilePanel | undefined {
      return this.focusedPanel?.type == "edit-file" ? (this.focusedPanel as EditFilePanel) : undefined;
    },
    focusedStatementCk(): string | null {
      return this.focusedFile?.activeStatementCk ?? null;
    },
    focusedStatementId(): string | null {
      return getNodeIdFromCkMaybe(this.projectVersionId, this.focusedStatementCk, "Statement");
    },
    focusedGroup(): PanelGroup | undefined {
      if (this.focusedPanel?.groupId == null) return undefined;
      return this.group(this.focusedPanel?.groupId);
    },
    lastActiveNodeCk(): string | null {
      const panels = this.panels
        .filter((e) => e.type == "edit-file")
        .sort((a, b) => ((a.lastActiveAt ?? "") > (b.lastActiveAt ?? "") ? -1 : 1));
      if (panels.length == 0) return null;
      const panel = panels[0] as EditFilePanel;
      return panel.activeStatementCk ?? panel.fileCk;
    },
    lastActiveFileCk(): string | null {
      const panels = this.panels
        .filter((e) => e.type == "edit-file")
        .sort((a, b) => ((a.lastActiveAt ?? "") > (b.lastActiveAt ?? "") ? -1 : 1));
      if (panels.length == 0) return null;
      const panel = panels[0] as EditFilePanel;
      return panel.fileCk;
    },
    // appearance
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

    _removePanelFromGroup(panel: Panel): boolean {
      /** Remove panel from its group, return whether focus is lost now */
      if (panel.groupId == null) return false;
      const group = this.group(panel.groupId);
      if (group == null) return false;
      group.panels = group.panels.filter((e) => e != panel);
      panel.groupId = null;
      if (group.activePanelId == panel.id) {
        // if active panel was removed, set last focused panel as active
        const nextToFocus = group.panels
          .slice()
          .sort((a, b) => ((a.lastActiveAt ?? "") > (b.lastActiveAt ?? "") ? -1 : 1))[0];
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
          } else {
            return true;
          }
        }
      }
      return false;
    },

    _coalesceClosedGroups(): void {
      // right now just move right group panels to left group if left group is empty
      if (this.keepEmptyPanelGroups) return;
      if (this.left.panels.length == 0 && this.right.panels.length > 0) {
        this.right.panels.forEach((e) => this.movePanel(e, this.left));
      }
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
      if (!this.recentlyClosedPanels.find((e) => e.id == panel.id)) {
        if (this.recentlyClosedPanels.length >= RECENTLY_CLOSED_PANELS_LIMIT) {
          this.recentlyClosedPanels.shift();
        }
        this.recentlyClosedPanels.push(panel.copy());
      }
      if (panel.groupId != null) {
        const focusLost = this._removePanelFromGroup(panel);
        // move focus to remaining panel group if focus was lost
        if (focusLost) {
          const nextToFocus = this.panels
            .slice()
            .sort((a, b) => ((a.lastFocusedAt ?? "") > (b.lastFocusedAt ?? "") ? -1 : 1))[0];
          if (nextToFocus != null) {
            this.focusPanel(nextToFocus);
          }
        }
      }
      this._coalesceClosedGroups();
    },

    closePanelGroup(group: PanelGroup): void {
      console.log(`close panel group ${group.id}`);
      group.panels.forEach((e) => this.closePanel(e));
    },

    reopenLastClosedPanel(options?: { focus: boolean }): Panel | undefined {
      const panel = this.recentlyClosedPanels.pop();
      if (panel == null) return;
      console.log(`reopen last closed panel ${panel.path} in group ${panel.groupId}`);
      const group = this.groups.find((g) => g.id == panel.groupId) || this.left;
      this.openPanel(panel, group);
      group.panels.push(panel);
      if (options?.focus) {
        this.focusPanel(panel);
      }
      return panel;
    },

    movePanel(panel: Panel, group: PanelGroup, options?: { copy?: boolean }): void {
      const wasFocused = panel.id == this.focusedPanelId;
      if (options?.copy) {
        panel = panel.copy({ resetId: true });
      }
      this.openPanel(panel, group);
      if (wasFocused) {
        this.focusPanel(panel);
      }
      this._coalesceClosedGroups();
    },

    _openMaybeCreate(filter: (panel: Panel) => boolean, create: () => Panel, options?: PanelOpenOptions): Panel {
      let panel = this.panels.find(filter);
      const created = panel == null;

      // try to recover panel state from recently closed
      if (!panel && !options?.create) {
        panel = this.recentlyClosedPanels.find(filter);
        if (panel) {
          console.log(`recover panel ${panel.path} from recently closed`);
          this.recentlyClosedPanels = this.recentlyClosedPanels.filter((e) => e.id != panel?.id);
          panel.groupId = null; // reset
        }
      }
      // create panel if it doesn't exist or forced
      if (!panel || options?.create) {
        panel = create();
        console.log(`create new panel ${panel.path}`);
        panel.onInstantiated(this);
      }
      let group = options?.group;
      if (!created) {
        // keep current group if panel already exists
        group = panel.group;
      } else if (options?.opposite) {
        group = this.nextGroup(group ?? this.focusedGroup ?? this.left);
      }
      this.openPanel(panel, group);
      if (options?.focus) {
        this.focusPanel(panel);
      }
      return panel;
    },

    openEditFile(file: NodeBase, options?: PanelOpenOptions): Panel {
      return this._openMaybeCreate(
        (p) => p.type == "edit-file" && (p as EditFilePanel).fileCk == file.ck,
        () => new EditFilePanel(file),
        options
      );
    },

    openEditDatabase(statement: NodeBase, options?: PanelOpenOptions): Panel {
      return this._openMaybeCreate(
        (p) => p.type == "edit-database" && (p as EditDatabasePanel).statementCk == statement.ck,
        () => new EditDatabasePanel(statement),
        options
      );
    },

    openLaunchRun(statement: { ck: string; name?: string | null }, options?: PanelOpenOptions): Panel {
      return this._openMaybeCreate(
        (p) => p.type == "launch-run" && (p as LaunchRunPanel).statementCk == statement.ck,
        () => new LaunchRunPanel(statement),
        options
      );
    },

    openViewRuns(query?: RunsQuery, options?: PanelOpenOptions): Panel {
      return this._openMaybeCreate(
        (p) => p.type == "view-runs",
        () => new ViewRunsPanel(query),
        options
      );
    },

    openViewRun(run: { id: string }, options?: PanelOpenOptions): Panel {
      return this._openMaybeCreate(
        (p) => p.type == "view-run" && (p as ViewRunPanel).runId == run.id,
        () => new ViewRunPanel(run),
        options
      );
    },

    openTerminal(options?: PanelOpenOptions): Panel {
      return this._openMaybeCreate(
        (p) => p.type == "terminal",
        () => new TerminalPanel(),
        options
      );
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
      const panel = this.openEditFile(file, { group });
      this.focusPanel(panel);
      return panel;
    },

    focusNode(node: NodeBase, group?: PanelGroup): Panel {
      if (node.__typename == "File") {
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
    recentlyClosedPanels: bench.$state.recentlyClosedPanels.map((e) => stripPanel(e)),
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
  bench.recentlyClosedPanels = bench.recentlyClosedPanels.map((e) => instantiate(e, bench));
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
      bench.projectHeadId = null;
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

export function getPanelActions(panel: Panel, bench: ReturnType<typeof useBenchState>) {
  const actions: PanelAction[] = [];

  // open other panels in this group if there are any
  if (!bench.showPanelTabs) {
    panel.group?.panels.forEach((e) => {
      if (e.id == panel.id) return;
      actions.push({
        groupId: "jump",
        label: e.name.length > 0 ? e.name : "(Unnamed)",
        icon: PANEL_ICONS_OUTLINE[e.type],
        action: () => bench.focusPanel(e),
      });
    });
  }

  // close / reopen
  const closeActions: PanelAction[] = [
    {
      groupId: "close",
      label: "Close",
      icon: XCircleIcon,
      action: () => bench.closePanel(panel),
    },
    {
      groupId: "close",
      label: "Close Others",
      icon: XCircleIcon,
      action: () => panel.group?.panels.filter((e) => e != panel).forEach((e) => bench.closePanel(e)),
    },
    {
      groupId: "close",
      label: "Close All",
      icon: XCircleIcon,
      action: () => bench.closePanelGroup(panel.group as PanelGroup),
    },
    {
      groupId: "close",
      label: "Reopen Closed",
      icon: ArrowUturnLeftIcon,
      disabled: bench.recentlyClosedPanels.length == 0,
      action: () => bench.reopenLastClosedPanel({ focus: true }),
    },
  ];
  actions.push(...closeActions);

  // view
  if (panel.effectiveWide) {
    actions.push({
      groupId: "view",
      label: "Narrow",
      icon: ArrowsPointingInIcon,
      hideInline: true,
      action: () => (panel.appearance.wide = false),
    });
  } else {
    actions.push({
      groupId: "view",
      label: "Expand",
      icon: ArrowsPointingOutIcon,
      hideInline: true,
      action: () => (panel.appearance.wide = true),
    });
  }

  // move
  // should clean this up
  if (panel.groupId == bench.left.id) {
    actions.push({
      groupId: "move",
      label: "Move Right",
      icon: ArrowRightIcon,
      action: () => bench.movePanel(panel, bench.right),
    });
    actions.push({
      groupId: "move",
      label: "Split Right",
      icon: ArrowRightIcon,
      action: () => bench.movePanel(panel, bench.right, { copy: true }),
    });
  } else {
    actions.push({
      groupId: "move",
      label: "Move Left",
      icon: ArrowLeftIcon,
      action: () => bench.movePanel(panel, bench.left),
    });
    actions.push({
      groupId: "move",
      label: "Split Left",
      icon: ArrowLeftIcon,
      action: () => bench.movePanel(panel, bench.left, { copy: true }),
    });
  }
  return actions;
}

export function getPanelGroupActions(group: PanelGroup, bench: ReturnType<typeof useBenchState>) {
  const actions: PanelGroupAction[] = [];

  // close
  actions.push({
    groupId: "close",
    label: "Close All",
    icon: XCircleIcon,
    action: () => bench.closePanelGroup(group),
  });
  actions.push({
    groupId: "close",
    label: "Reopen Closed",
    icon: ArrowUturnLeftIcon,
    disabled: bench.recentlyClosedPanels.length == 0,
    action: () => bench.reopenLastClosedPanel({ focus: true }),
  });

  // 'move' (merge) actions
  if (bench.groups.filter((g) => g.panels.length > 0).length > 1) {
    if (group.id == bench.left.id) {
      actions.push({
        groupId: "move",
        label: "Merge Right",
        icon: ArrowRightIcon,
        action: () => group.panels.forEach((e) => bench.movePanel(e, bench.right)),
      });
    } else {
      actions.push({
        groupId: "move",
        label: "Merge Left",
        icon: ArrowLeftIcon,
        action: () => group.panels.forEach((e) => bench.movePanel(e, bench.left)),
      });
    }
  }
  return actions;
}

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
    actions: computed(() => getPanelActions(panel.value, bench)),
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
  component?: (item: T) => { props: any; component: InstanceType<any> };
  active?: boolean;
  disabled?: boolean;
  keepOpen?: boolean;
  hideInline?: boolean;
  hideInMenu?: boolean;
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
export type PanelGroupAction = Action<PanelGroup>;

// specific panels

export type NavElementType = "Statement" | "Field" | "Record";
export type NavElement = { id: string; ck: string; __typename?: NavElementType };

export abstract class NavigablePanel extends Panel {
  activeStatementCk?: string;
  selectedElementType?: NavElementType;
  selectedElementIds: string[] = [];
  elementProperties: Record<string, any> = {};
  editing = false;

  onInstantiated(bench: ReturnType<typeof useBenchState>) {
    super.onInstantiated(bench);
    this.elementProperties = this.elementProperties || {};
  }

  focusElement(element: NavElement, retainEditing = false) {
    if (element.__typename != "Statement") {
      throw new Error(`focusElement only supports Statement elements, got ${element.__typename}`);
    }
    if (this.activeStatementCk == element.ck) return;
    console.debug(`focus element ${element.ck}`);
    this.activeStatementCk = element.ck;
    this.editing = this.editing && retainEditing;
  }

  blurElement(element?: NavElement) {
    if (element == null || element.id == this.activeStatementCk) {
      this.activeStatementCk = undefined;
      this.editing = false;
      console.debug(`blur element ${element?.id} ${element?.ck}`);
    }
  }

  editElement(element: NavElement) {
    this.focusElement(element);
    this.editing = true;
    console.debug(`edit element ${element.id} ${element.ck}`);
  }

  stopEditingElement(element?: NavElement) {
    if (element == null || element.ck == this.activeStatementCk) {
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
  fileCk: string;
  foldedStatementContentCks?: string[] = [];

  constructor(file: NodeBase) {
    super(
      "edit-file",
      file.ck + "-" + randomHexString(),
      file.name ?? "(Unnamed)",
      (file.name ?? "(Unnamed)") + "-" + file.ck.replace(/-/g, ""),
      null
    );
    this.fileCk = file.ck;
  }

  resetId(): void {
    this.id = this.fileCk + "-" + randomHexString();
  }

  editElement(element: NavElement, focus = true) {
    super.editElement(element);
    if (focus) {
      nextTick(() => {
        (this.component as InstanceType<typeof EditFilePanelVue>)?.statementsComponents[element.id]?.focus();
      });
    }
  }

  isStatementContentFolded(statement: { ck: string }): boolean {
    return this.foldedStatementContentCks?.includes(statement.ck) ?? false;
  }

  toggleStatementContentFolded(statement: { ck: string }): void {
    if (this.isStatementContentFolded(statement)) {
      this.foldedStatementContentCks = this.foldedStatementContentCks?.filter((id) => id != statement.ck);
    } else {
      this.foldedStatementContentCks = this.foldedStatementContentCks ?? [];
      this.foldedStatementContentCks.push(statement.ck);
    }
  }

  setStatementContentsFolded(statements: { ck: string }[], folded: boolean): void {
    if (folded) {
      this.foldedStatementContentCks = [...(this.foldedStatementContentCks ?? []), ...statements.map((s) => s.ck)];
    } else {
      this.foldedStatementContentCks = this.foldedStatementContentCks?.filter(
        (ck) => !statements.find((s) => s.ck == ck)
      );
    }
  }

  updatePath(fileHeader: { id: string; ck: string; name?: string | null }, module: ModuleIndex) {
    const file = module.filesById[fileHeader.id];
    if (file == null) return;
    this.name = fileHeader.name ?? file.name;
    this.path = (file.name ?? "(Unnamed)") + "-" + file.ck.replace(/-/g, "");
  }

  static parsePath(path: string, module: ModuleIndex): Panel | null {
    let matchingFile = null;
    try {
      const ck = dashifyUuid(path.split("-").slice(-1)[0]);
      matchingFile = module.filesById[module.idByCk[ck]];
    } catch (e) {
      // ignore
    }
    // try to match by name
    matchingFile = matchingFile ?? Object.values(module.filesById).find((f) => path.startsWith(prettifySlug(f.name)));
    if (matchingFile == null) return null;
    return new EditFilePanel(matchingFile as NodeBase);
  }
}

export class EditDatabasePanel extends NavigablePanel {
  type = "edit-database" as const;
  statementCk: string;
  sorts?: Sort[];
  filters?: Conditional[];
  inlineQuery?: string;
  wrap?: boolean = true;

  constructor(statement: { ck: string; name?: string | null }) {
    super(
      "edit-database",
      statement.ck + "-" + randomHexString(),
      statement.name ?? "(Unnamed)",
      (statement.name ?? "(Unnamed)") + "-" + statement.ck.replace(/-/g, "")
    );
    this.statementCk = statement.ck;
  }

  resetId(): void {
    this.id = this.statementCk + "-" + randomHexString();
  }

  updatePath(statementHeader: { id: string; ck: string; name?: string | null }, module: ModuleIndex) {
    const statement = module.statementsById[statementHeader.id];
    if (statement == null) return;
    this.name = statementHeader.name ?? statement.name ?? "";
    this.path = (statement.name ?? "(Unnamed)") + "-" + statement.ck.replace(/-/g, "");
  }

  static parsePath(path: string, module: ModuleIndex): Panel | null {
    let matchingStatement = null;
    try {
      const ck = dashifyUuid(path.split("-").slice(-1)[0]);
      matchingStatement = module.statementsById[module.idByCk[ck]];
    } catch (e) {
      // ignore
    }
    if (matchingStatement == null) return null;
    return new EditDatabasePanel(matchingStatement);
  }

  get hasWhiteBackground() {
    return true;
  }

  get hasScrollY() {
    return false;
  }
}

export class LaunchRunPanel extends Panel {
  type = "launch-run" as const;
  statementCk: string;
  statementType?: StatementType.Task | StatementType.Code;
  inputs: Record<string, any> = {};
  lastOutput?: Record<string, any> | null = null;
  lastError?: RunError | null = null;
  lastRunTerminatedAt?: string;
  lastRunId?: string;
  lastSessionId?: string;

  constructor(statement: { ck: string; name?: string | null; __typename?: string }) {
    super(
      "launch-run",
      statement.ck + "-" + randomHexString(),
      statement.name ?? "(Unnamed)",
      (statement.name ?? "(Unnamed)") + "-" + statement.ck.replace(/-/g, "") + "@launch-run"
    );
    this.statementCk = statement.ck;
    if (statement.__typename == "Task") {
      this.statementType = StatementType.Task;
    } else if (statement.__typename == "Code") {
      this.statementType = StatementType.Code;
    }
  }

  resetId(): void {
    this.id = this.statementCk + "-" + randomHexString();
  }

  updatePath(statementHeader: { id: string; name?: string | null }, module: ModuleIndex) {
    const statement = module.statementsById[statementHeader.id];
    const file = module.filesById[statement?.file?.id ?? ""];
    if (statement == null || file == null) return;
    this.name = statementHeader.name ?? statement.name ?? "";
    this.path = `${statement.name ?? "(Unnamed)"}-${statement.ck.replace(/-/g, "")}@${this.type}`;
  }

  static parsePath(path: string, module: ModuleIndex): Panel | null {
    let matchingStatement = null;
    try {
      const ck = dashifyUuid(path.split("-").slice(-1)[0]);
      matchingStatement = module.statementsById[module.idByCk[ck]];
    } catch (e) {
      // ignore
    }
    // try to match by name
    matchingStatement =
      matchingStatement ??
      Object.values(module.statementsById).find((s) => path.startsWith(prettifySlug(s.name ?? "(Unnamed)")));
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
  sort?: Sort;

  constructor(query?: RunsQuery, sort?: Sort) {
    super("view-runs", "runs-" + randomHexString(), "Runs", "runs");
    this.limit = 30;
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
      "run:" + getUUIDFromGlobalID(run.id)
    );
    this.runId = run.id;
  }

  resetId(): void {
    this.id = this.runId + "-" + randomHexString();
  }

  static parsePath(path: string, module: ModuleIndex): Panel | null {
    const [kind, runId] = path.split(":");
    if (kind != "run" || !isValidUUID(runId)) return null;
    return new ViewRunPanel({ id: toGlobalId("Run", runId) });
  }

  get hasWhiteBackground() {
    return false;
  }
}

export class ViewLogsPanel extends Panel {
  type = "view-logs" as const;
  query?: LogsQuery;
  sort?: Sort;

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

export class TerminalPanel extends Panel {
  type = "terminal" as const;
  code = "";
  runMode: "approve" | "immediate" = "approve";
  lastRunId?: string;
  accessLevel: SessionAccessLevel = SessionAccessLevel.Update;
  clearedAt?: string;

  constructor() {
    super("terminal", "terminal-" + randomHexString(), "Terminal", "Terminal");
  }

  resetId(): void {
    this.id = "terminal-" + randomHexString();
  }

  clearHistory() {
    this.clearedAt = new Date().toISOString();
  }

  restoreHistory() {
    this.clearedAt = undefined;
  }

  static parsePath(path: string, module: ModuleIndex): Panel | null {
    return null;
  }

  get hasWhiteBackground() {
    return false;
  }

  get hasScrollY() {
    return false;
  }
}

export const PANEL_INSTANCE_TYPES: Record<PanelType, typeof Panel> = {
  "edit-file": EditFilePanel as any,
  "edit-database": EditDatabasePanel as any,
  "launch-run": LaunchRunPanel as any, // don't care about constructor type
  "view-runs": ViewRunsPanel as any,
  "view-run": ViewRunPanel as any,
  "view-logs": ViewLogsPanel as any,
  terminal: TerminalPanel as any,
};

export const PANEL_ICONS_OUTLINE: Record<PanelType, any> = {
  "edit-file": CodeBracketIconOutline,
  "edit-database": CircleStackIconOutline,
  "launch-run": WindowIconOutline,
  "view-runs": PlayIconOutline,
  "view-run": PlayIconOutline,
  "view-logs": Bars4IconOutline,
  terminal: CommandLineIconOutline,
};

export const PANEL_ICONS_SOLID: Record<PanelType, any> = {
  "edit-file": CodeBracketIconSolid,
  "edit-database": CircleStackIconSolid,
  "launch-run": WindowIconSolid,
  "view-runs": PlayIconSolid,
  "view-run": PlayIconSolid,
  "view-logs": Bars4IconSolid,
  terminal: CommandLineIconSolid,
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
  panel.onInstantiated(bench);
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

type BenchVersioning = {
  project: Ref<ProjectHeader | null>;
  currentVersion: Ref<ProjectVersionHeader | null>;
};

export function provideBenchVersioning(project: Ref<ProjectHeader | null>, currentVersion: Ref<ProjectVersionHeader>) {
  provide<BenchVersioning>("bench-versioning", { project, currentVersion });
}

export function useBenchVersioning() {
  const versioning = inject<BenchVersioning>("bench-versioning");
  if (versioning == null) {
    throw new Error("bench versioning not provided");
  }
  const bench = useBenchState();
  const notifications = useNotifications();

  async function restoreNode(ck: string) {
    // TODO @Feature: restore nodes (see :BE-399)
  }

  return {
    isAtHead: toRef(bench, "isAtHead"),
    currentVersionName: computed(
      () => versioning.currentVersion.value?.name ?? versioning.currentVersion.value?.name ?? "???"
    ),
    restoreNode,
  };
}
