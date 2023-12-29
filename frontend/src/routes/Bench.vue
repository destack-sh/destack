<script setup lang="ts">
import ClientsPopover from "@/components/basic/ClientsPopover.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import FatHeader from "@/components/basic/FatHeader.vue";
import GenericNotFound from "@/components/basic/GenericNotFound.vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import NotificationArea from "@/components/basic/NotificationArea.vue";
import OmniCreate from "@/components/basic/OmniCreate.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import HelpPopover from "@/components/bench/HelpPopover.vue";
import NotificationPopover from "@/components/bench/NotificationPopover.vue";
import ProjectPopover from "@/components/bench/ProjectPopover.vue";
import SettingsPopover from "@/components/bench/SettingsPopover.vue";
import PanelGroup from "@/components/panels/PanelGroup.vue";
import ViewExplorer from "@/components/views/ViewExplorer.vue";
import ViewHistory from "@/components/views/ViewHistory.vue";
import ViewIssues from "@/components/views/ViewIssues.vue";
import { graphql, useFragment } from "@/gql";
import { WorkerSetStatus } from "@/gql/graphql";
import { provideAction, useActions } from "@/state/actions";
import { ModuleAccessLevel, base64ToUuid, useAuth } from "@/state/auth";
import {
  PANEL_INSTANCE_TYPES,
  prettifySlug,
  PROJECT_ACCESS_LEVEL_NAME,
  provideBenchVersioning,
  useBenchPersistence,
  useBenchState,
  type ProjectHeader,
  type ViewId,
  type ProjectVersionHeader,
} from "@/state/bench";
import { ProjectHeaderType, ProjectVersionHeaderType } from "@/state/fragments";
import { useCurrentModule, type ModuleIndex } from "@/state/module";
import { useNotifications } from "@/state/notifications";
import { useOperationsStore } from "@/state/operations";
import { useModuleSync, useProjectSync } from "@/state/sync";
import { ACTIVE_SHARING_TOKEN, WS_CONNECTED } from "@/utils/globals";
import { PopoverButton } from "@headlessui/vue";
import {
  ClockIcon as ClockIconOutline,
  Cog8ToothIcon,
  CubeIcon,
  DocumentDuplicateIcon,
  QuestionMarkCircleIcon,
  ExclamationTriangleIcon as ExclamationTriangleIconOutline,
} from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useElementSize, useTitle, useWindowFocus, useWindowSize } from "@vueuse/core";
import Mousetrap from "mousetrap";
import {
  computed,
  onBeforeUnmount,
  ref,
  toRef,
  watch,
  watchEffect,
  type Component,
  type ComputedRef,
  type Ref,
  nextTick,
} from "vue";
import { useRouter } from "vue-router";
import { getUUIDFromGlobalID } from "@/utils/functools";
import {
  EyeIcon as EyeIconSolid,
  SignalIcon,
  SignalSlashIcon,
  XCircleIcon,
  ExclamationTriangleIcon as ExclamationTriangleIconSolid,
  ArrowLeftIcon,
  ArrowRightIcon,
  HomeIcon as HomeIconSolid,
  CubeIcon as CubeIconSolid,
  InformationCircleIcon,
  CommandLineIcon,
  ClockIcon,
} from "@heroicons/vue/24/solid";
import ViewEnvironment from "@/components/views/ViewEnvironment.vue";
import CurrentRunsPopover from "@/components/bench/CurrentRunsPopover.vue";
import { WORKER_STATUS_COLOR, WORKER_STATUS_TITLE, useCurrentSessions } from "@/state/session";
import SharingPopover from "@/components/bench/SharingPopover.vue";
import type { Project, ProjectVersion } from "@/gql/graphql";
import { useElementRefs } from "@/composables/useGrid";
import { DateTime } from "luxon";
import { AggregationOp } from "@/wire";

const props = defineProps<{
  owner: string;
  project: string;
  version?: string;
}>();

// set secret sharing token if present in url (must be done first)
const router = useRouter();
watch(
  () => router.currentRoute.value.query,
  () => {
    if (router.currentRoute.value.query.s != null) {
      ACTIVE_SHARING_TOKEN.value = base64ToUuid(router.currentRoute.value.query.s as string);
      console.debug("using sharing token from url", ACTIVE_SHARING_TOKEN.value);
    } else {
      ACTIVE_SHARING_TOKEN.value = null;
    }
  },
  { immediate: true }
);

// views for the sidebar
type View = {
  id: ViewId;
  label: string;
  icon: Component;
  view: Component;
  enabled: boolean;
};
const views: Ref<View[]> = computed(() => [
  { id: "explorer", label: "Explorer", icon: DocumentDuplicateIcon, view: ViewExplorer, enabled: true },
  { id: "history", label: "History", icon: ClockIconOutline, view: ViewHistory, enabled: true },
  { id: "issues", label: "Issues", icon: ExclamationTriangleIconOutline, view: ViewIssues, enabled: true },
  { id: "environment", label: "Environment", icon: CubeIcon, view: ViewEnvironment, enabled: true },
]);
const viewsRefs = useElementRefs<InstanceType<typeof ViewExplorer> | null>(views);
const activeView: ComputedRef<View> = computed(() => {
  const view = views.value.find((v) => v.id == bench.activeViewId);
  if (!view) {
    console.error("invalid view id: " + bench.activeViewId);
    bench.setActiveView(views.value[0].id);
    return views.value[0];
  }
  return view;
});

function toggleActiveView(viewId: ViewId, ignoreFocus: boolean) {
  if (bench.activeViewId == viewId && bench.showViewContent && (ignoreFocus || bench.focusedViewId == viewId)) {
    bench.showViewContent = false;
    bench.focusedViewId = null;
  } else {
    bench.focusView(viewId);
  }
}
for (const view of views.value) {
  provideAction({
    id: `bench.view.open${view.id}`,
    label: `View ${view.label}`,
    shortcuts: [`alt+${views.value.indexOf(view) + 1}`],
    apply: () => toggleActiveView(view.id, false),
  });
}

// other buttons for sidebar
type SidebarPopover = {
  component: Component;
  icon: Component;
  label: string;
};
const sidebarPopovers: SidebarPopover[] = [
  {
    component: HelpPopover,
    icon: QuestionMarkCircleIcon,
    label: "Help",
  },
  {
    component: SettingsPopover,
    icon: Cog8ToothIcon,
    label: "Settings",
  },
];

// get project header
const {
  error: projectError,
  result: projectResult,
  loading: projectLoading,
} = useQuery(
  graphql(/* GraphQL */ `
    query projectBySlug($owner: String!, $project: String!) {
      projectBySlug(owner: $owner, project: $project) {
        ...ProjectHeader
      }
    }
  `),
  () => ({
    owner: props.owner,
    project: props.project,
  })
);
const projectLoaded = computed(() => !!projectResult.value?.projectBySlug);
const projectFetched = computed(() => useFragment(ProjectHeaderType, projectResult.value?.projectBySlug));
const projectHead = computed(() => useFragment(ProjectVersionHeaderType, projectFetched.value?.head));

// default version to view = head (can be overridden by URL?)
const versionToViewId = computed(() => {
  if (props.version != null) {
    return props.version;
  } else {
    return projectHead.value?.id;
  }
});

// set up bench state
const bench = useBenchState();
const ready = computed(() => bench.projectVersionId != null && bench.projectVersionId == versionToViewId.value);
const notifications = useNotifications();
const viewContainerRef = ref<HTMLElement | null>(null);
const viewContainerSize = useElementSize(viewContainerRef);
const { height: windowHeight } = useWindowSize();
const mainContainerRef = ref<HTMLElement | null>(null);

// sync title bar with project info
const title = useTitle();
watchEffect(() => {
  if (projectError.value) {
    title.value = "Page not found";
  } else {
    if (bench.focusedPanel != null) {
      title.value = (bench.focusedPanel.name || "(Unnamed)") + " • " + `${props.owner}/${props.project}`;
    } else {
      title.value = `${props.owner}/${props.project}${projectFetched.value ? " • " + projectFetched.value.name : ""}`;
    }
  }
});

// get project content
// TODO @Performance: consolidate project version load into project load (if version to view == head)
const { error: versionError, result: versionResult } = useQuery(
  graphql(/* GraphQL */ `
    query projectVersionHeader($id: GlobalID!) {
      projectVersion(id: $id) {
        id
        ck
        name
        tag
        description
        createdAt
        committed
        committedAt
        parents {
          id
        }
        children {
          id
        }
      }
    }
  `),
  () => ({ id: versionToViewId.value }),
  () => ({ enabled: !!versionToViewId.value })
);
const versionFetched = computed(() => versionResult.value?.projectVersion);
const versionLoaded = computed(() => !!versionFetched.value);

// react to version load error
watch(versionError, () => {
  if (versionError.value != null) {
    const atHead = versionFetched.value?.id == projectHead.value?.id;
    notifications.show({
      kind: "error",
      type: "version.loadFailed",
      message: "Version unavailable",
      description: "Failed to load version.",
    });
    if (!atHead) {
      // revert to head
      router.replace({ hash: router.currentRoute.value.hash });
    } // otherwise there's nothing we can do (?)
  }
});

// actions (ensure global actions are available)
const actions = useActions();
const operationsStore = useOperationsStore();
const module = useCurrentModule();
const sessions = useCurrentSessions();
const auth = useAuth();
// syncs need to instantiated for the Bench lifetime
useModuleSync(toRef(bench, "projectId"), versionToViewId);
useProjectSync(toRef(bench, "projectId"));

// status
const workerSet = sessions.workerSet;
const hasStaleInflightOps = computed(() => operationsStore.hasInflightLike({ stateless: false, stale: true }));
const hasInflightOps = computed(() => operationsStore.hasInflightLike({ stateless: false }));
const connectionHealthy = computed(() => WS_CONNECTED.value && !hasStaleInflightOps.value);
const workerSetHealthy = computed(() => workerSet.value?.status == WorkerSetStatus.Healthy);

const lastWakeAttemptAt = ref<DateTime | null>(null);
const windowFocus = useWindowFocus();
// automatically wake workers on load and when focused (if not already awake)
watch(
  () => [bench.canUse, windowFocus.value, workerSet.value?.status],
  () => {
    if (!windowFocus.value) {
      lastWakeAttemptAt.value = null;
    }
    if (bench.canUse && windowFocus.value && workerSet.value?.status == WorkerSetStatus.Sleeping) {
      const secondsSinceLastAttempt = lastWakeAttemptAt.value?.diffNow("seconds").seconds ?? Infinity;
      if (secondsSinceLastAttempt >= 20) {
        console.log("auto wake workers");
        sessions.wakeWorkerSet();
        lastWakeAttemptAt.value = DateTime.now();
      }
    }
  },
  { immediate: true }
);

// prevent close if there are inflight ops
// TODO @UX @Robustness: prompt if unsaved changes doesn't always work
const confirmDiscardUnsaved = (e: Event) => {
  if (hasInflightOps.value) {
    e.preventDefault();
    return "Your Bench has unsaved changes. Are you sure you want to leave?";
  } else {
    return undefined;
  }
};
window.addEventListener("beforeunload", confirmDiscardUnsaved);
onBeforeUnmount(() => {
  document.body.removeEventListener("beforeunload", confirmDiscardUnsaved);
});

// routing
const consideredUrl = ref(false);
// open/focus editor from url if hash changes and none is open (once)
watch(
  () => [module.idx.value, router.currentRoute.value],
  () => {
    if (module.idx.value != null && !consideredUrl.value && ready.value) {
      const hash = router.currentRoute.value.hash.slice(1);
      if (hash.length != 0) {
        const panel = Object.values(PANEL_INSTANCE_TYPES)
          .map((panelType) => panelType.parsePath(hash, module.idx.value as ModuleIndex))
          .find((e) => e != null);
        if (panel != null) {
          panel.onInstantiated(bench);
          if (!bench.panels.some((p) => p.path == panel.path)) {
            // open from url if not already open
            bench.openPanel(panel);
            bench.focusPanel(panel);
            console.log(`open ${panel.path} (${panel.type}) from url`);
          }
        } else {
          console.debug(`no matching editor for ${hash}`);
        }
      }
      consideredUrl.value = true;
    }
  },
  { immediate: true }
);

// change url if focused editor changes
watchEffect(() => {
  if (bench.focusedPanel != null) {
    // set hash to open path
    if (consideredUrl.value) {
      const prettyPath = prettifySlug(bench.focusedPanel.path);
      router.replace({ hash: `#${prettyPath}`, query: router.currentRoute.value.query });
    }
  } else if (ready.value && consideredUrl.value) {
    // clear hash
    router.replace({ hash: ``, query: router.currentRoute.value.query });
  }
});

// snapshot on ctrl+s (open history view & focus snapshot modal)
// maybe directly show global snapshot modal instead
Mousetrap.bind(["ctrl+s", "meta+s"], () => {
  if (bench.activeViewId != "history" || !bench.showViewContent) {
    bench.focusView("history");
    nextTick(() => (viewsRefs.getRef("history") as InstanceType<typeof ViewHistory>).open?.());
  } else {
    (viewsRefs.getRef("history") as InstanceType<typeof ViewHistory>).open?.();
  }
  return false;
});

// show notification if disconnected/reconnected
const disconnected = ref(false);
const wasEverConnected = ref(false);
watch(
  () => [WS_CONNECTED.value],
  () => {
    if (WS_CONNECTED.value) {
      wasEverConnected.value = true;
    }
    if (!WS_CONNECTED.value && !disconnected.value && wasEverConnected.value) {
      notifications.show({
        type: "runtime.disconnected",
        kind: "warning",
        message: "Disconnected",
      });
      disconnected.value = true;
    } else if (WS_CONNECTED.value && disconnected.value) {
      disconnected.value = false;
      notifications.show({
        type: "runtime.reconnected",
        kind: "success",
        message: "Reconnected",
      });
      notifications.dismissIf({ type: "runtime.disconnected" });
    }
  }
);

const { load } = useBenchPersistence();

// manage read/write access
watchEffect(() => {
  bench.readonly =
    !versionLoaded.value ||
    versionToViewId.value != projectHead.value?.id ||
    ![ModuleAccessLevel.Admin, ModuleAccessLevel.Manage, ModuleAccessLevel.Edit].includes(
      bench.accessLevel as ModuleAccessLevel
    ) ||
    versionFetched.value?.committed == true;
});

// prepare/update bench state whenever project changes
watch(
  () => [projectFetched.value, versionToViewId.value, () => bench.projectId, () => bench.projectVersionId],
  () => {
    if (projectFetched.value != null && bench.projectId != projectFetched.value.id) {
      console.log(`load bench ${projectFetched.value.slug} (${projectFetched.value.id} at ${versionToViewId.value})`);
      const loaded = load(projectFetched.value.id);
      if (!loaded) {
        bench.$reset();
        console.log(`no local bench state available, reset bench ${projectFetched.value.slug}`);
      }
      bench.projectId = projectFetched.value.id;
      bench.projectHeadId = (projectFetched.value.head as ProjectVersion)?.id;
    }
    if (versionToViewId.value != null && bench.projectVersionId != versionToViewId.value) {
      bench.projectVersionId = versionToViewId.value;
    }
    bench.accessLevel = projectFetched.value?.accessLevel ?? null;
  },
  { immediate: true }
);

provideBenchVersioning(projectFetched as Ref<ProjectHeader | null>, versionFetched as Ref<ProjectVersionHeader>);

// clear bench state when exiting view
onBeforeUnmount(() => {
  if (bench.projectId == projectFetched.value?.id) {
    bench.$reset();
  }
});
</script>

<template>
  <!-- Root -->
  <!-- Crude min-w to make this not look totally shit on mobile -->
  <div class="relative flex h-full min-w-[800px] flex-col bg-gray-50">
    <!-- Header with controls and auth -->
    <FatHeader v-show="bench.showBenchHeader">
      <!-- Left side: organizational & status -->
      <template v-slot:left>
        <!-- Home -->
        <HomeButton />
        <!-- Project menu -->
        <div v-if="projectLoading || projectLoaded" class="ml-1.5 flex flex-row items-baseline whitespace-nowrap">
          <!-- Owner -->
          <router-link :to="`/${props.owner}`" class="rounded-sm p-1 text-sm hover:bg-orange-100">
            {{ props.owner }}
          </router-link>
          <span class="text-gray-500">/</span>
          <!-- Project button -->
          <ProjectPopover v-if="projectLoaded && projectFetched" :project="projectFetched">
            <template v-slot:button="{ open }">
              <PopoverButton
                class="flex h-full items-center justify-between rounded-sm bg-white p-1 text-left hover:bg-orange-100 focus:outline-none"
                :class="{ 'bg-orange-100 focus:bg-orange-100': open }"
              >
                <span class="truncate text-sm">{{ props.project }}</span>
              </PopoverButton>
            </template>
          </ProjectPopover>
          <!-- While loading, imitate project button -->
          <span v-else class="truncate p-1 text-sm">
            {{ props.project }}
          </span>
          <!-- Show ids for debugging (if enabled) -->
          <div
            v-if="bench.debug && bench.projectId && bench.projectVersionId"
            class="left-18 absolute top-0 z-20 rounded-sm bg-red-200 bg-opacity-50 font-sans text-xs"
          >
            <span class="select-all">{{ getUUIDFromGlobalID(bench.projectId) }}</span> /
            <span class="select-all">{{ getUUIDFromGlobalID(bench.projectVersionId) }}</span>
            ({{ PROJECT_ACCESS_LEVEL_NAME[bench.accessLevel as ModuleAccessLevel] }})
          </div>
          <!-- Version info (if not at head) -->
          <FadeTransition>
            <div
              v-if="versionToViewId != projectHead?.id && versionFetched != null"
              class="ml-1.5 flex flex-row rounded-sm border border-orange-900/[12%] bg-yellow-600 px-2 py-0.5 text-sm text-white"
            >
              <span class="relative">
                <ClockIcon class="absolute top-0 h-5 w-5 text-white hover:text-yellow-200" />
                <span class="ml-6 font-bold">{{ versionFetched?.tag ?? versionFetched?.name ?? "Autosave" }}</span>
              </span>
              <router-link :to="{ hash: router.currentRoute.value.hash }" class="ml-2.5 flex items-center">
                <HomeIconSolid class="mr-0.5 h-4 w-4 text-white hover:text-gray-200" />
              </router-link>
              <router-link
                :to="{ hash: router.currentRoute.value.hash, query: { version: versionFetched.parents[0]?.id } }"
                class="ml-1 flex items-center"
                :class="versionFetched.parents.length == 0 ? 'text-gray-700' : 'text-white hover:text-gray-200'"
                :disabled="versionFetched.parents.length == 0"
              >
                <ArrowLeftIcon class="mr-0.5 h-4 w-4" />
              </router-link>
              <router-link
                :to="{ hash: router.currentRoute.value.hash, query: { version: versionFetched.children[0]?.id } }"
                class="ml-0 flex items-center"
                :class="versionFetched.children.length == 0 ? 'text-gray-700' : ' text-white hover:text-gray-200'"
                :disabled="versionFetched.children.length == 0"
              >
                <ArrowRightIcon class="mr-0.5 h-4 w-4" />
              </router-link>
              <!-- Full Bench restore not implemented yet -->
              <!-- <button
                v-if="bench.canEdit"
                class="ml-1 text-white underline decoration-dashed underline-offset-2 hover:text-gray-200 hover:decoration-solid"
              >
                Restore version
              </button> -->
            </div>
          </FadeTransition>
          <!-- Read-only project notice -->
          <div
            v-if="bench.accessLevel != null && !bench.canEdit"
            class="ml-1.5 flex flex-row gap-2 rounded-sm border border-orange-900/[12%] bg-orange-100 px-2 py-0.5 text-sm"
          >
            <span class="relative flex flex-row gap-1 text-gray-900">
              <EyeIconSolid class="top-0.0 absolute h-5 w-5 text-gray-500" />
              <span class="ml-6 select-none">{{ PROJECT_ACCESS_LEVEL_NAME[bench.accessLevel] }} only</span>
            </span>
          </div>
        </div>
        <!-- Comments, issues -->
        <div class="ml-1.5 flex flex-row gap-0.5" :class="versionLoaded ? 'visible' : 'hidden'">
          <!-- Notices -->
          <button
            class="flex items-center gap-0.5 rounded-sm p-1 hover:bg-orange-100"
            v-if="module.notices.value?.length || 0 > 0"
            @click="toggleActiveView('issues', true)"
          >
            <InformationCircleIcon class="h-5 w-5 text-cyan-600" />
            <span class="text-sm font-semibold text-gray-700">{{ module.notices.value?.length }}</span>
          </button>
          <!-- Warnings -->
          <button
            class="flex items-center gap-0.5 rounded-sm p-1 hover:bg-orange-100"
            v-if="module.errors.value?.length || 0 > 0"
            @click="toggleActiveView('issues', true)"
          >
            <XCircleIcon class="h-5 w-5 text-red-600" />
            <span class="text-sm font-semibold text-gray-700">{{ module.errors.value?.length }}</span>
          </button>
          <!-- Errors -->
          <button
            class="flex items-center gap-0.5 rounded-sm p-1 hover:bg-orange-100"
            v-if="module.warnings.value?.length || 0 > 0"
            @click="toggleActiveView('issues', true)"
          >
            <ExclamationTriangleIconSolid class="h-5 w-5 text-yellow-600" />
            <span class="text-sm font-semibold text-gray-700">{{ module.warnings.value?.length }}</span>
          </button>
        </div>
        <!-- Connection status -->
        <div v-if="versionLoaded && !connectionHealthy" class="ml-1.5 flex">
          <span
            class="cursor-pointer p-1 text-sm transition-colors duration-150 hover:bg-orange-100"
            :class="[!connectionHealthy ? 'animate-pulse text-yellow-600' : 'text-green-700']"
          >
            <component :is="connectionHealthy ? SignalIcon : SignalSlashIcon" class="h-4 w-4" />
          </span>
        </div>
        <!-- Worker status -->
        <div v-if="workerSet != null && !workerSetHealthy" class="ml-1.5 flex">
          <span
            class="group relative flex cursor-pointer items-center p-1 text-sm transition-colors duration-150 hover:bg-orange-100"
            :class="[
              workerSet.status == WorkerSetStatus.Pending || workerSet.status == WorkerSetStatus.Updating
                ? 'animate-pulse '
                : '',
              WORKER_STATUS_COLOR[workerSet.status],
            ]"
            @click="toggleActiveView('environment', true)"
          >
            <CubeIconSolid class="h-5 w-5" />
            <span v-if="!workerSetHealthy" class="ml-1">{{ WORKER_STATUS_TITLE[workerSet.status] }}</span>
            <span
              class="pointer-events-none absolute -left-4 top-7 z-30 w-fit whitespace-nowrap rounded-sm bg-white px-1.5 text-xs opacity-0 ring-1 ring-orange-900 ring-opacity-[25%] transition duration-75 group-hover:opacity-100"
            >
              Environment is {{ WORKER_STATUS_TITLE[workerSet.status].toLowerCase() }}
            </span>
          </span>
        </div>
      </template>

      <!-- Right side: controls & profile -->
      <template v-slot:right>
        <!-- Bench-global controls -->
        <FadeTransition>
          <div v-if="versionLoaded && projectFetched != null" class="flex h-full flex-row items-center space-x-2">
            <CurrentRunsPopover />
            <button
              class="rounded-sm p-1 hover:bg-orange-100"
              @click="
                bench.focusedPanel?.type == 'terminal'
                  ? bench.closePanel(bench.focusedPanel)
                  : bench.openTerminal({ group: bench.focusedGroup, focus: true })
              "
            >
              <CommandLineIcon class="h-5 w-5 text-orange-600" />
            </button>
            <SharingPopover :project="(projectFetched as any as Project)" />
            <NotificationPopover @show="bench.showBenchHeader = true" />
            <OmniCreate @show="bench.showBenchHeader = true" />
          </div>
        </FadeTransition>
        <ClientsPopover v-if="auth.loggedIn.value" class="ml-2" size="large" />
        <ProfileButton class="ml-2" />
      </template>
    </FatHeader>
    <!-- Main content (sidebar + editor), spans horizontally -->
    <div v-if="projectLoading" class="flex w-full flex-1 flex-col items-center justify-center">
      <BusySpinnerIcon class="h-8 w-8 animate-spin" />
    </div>
    <div v-show="projectLoaded" class="relative flex flex-1 flex-row">
      <!-- Sidebar of view buttons & views -->
      <aside
        class="flex h-full resize-x"
        :class="{
          'w-80': bench.showViewContent && bench.showViewSelection,
          'w-64': bench.showViewContent && !bench.showViewSelection,
        }"
      >
        <!-- View buttons -->
        <!-- inset 1px above to hide border bottom from top bar (experimental design tweak) -->
        <div
          class="-mt-[1px] flex h-full min-h-0 flex-col border-r border-orange-900/[12%] bg-white pt-[1px]"
          v-show="bench.showViewSelection"
        >
          <!-- Top of sidebar: view selection -->
          <div class="flex flex-1 flex-col">
            <button
              v-for="view in views"
              :key="view.id"
              class="group relative border-l-2 border-gray-50 px-2 py-2.5 text-gray-600 hover:bg-orange-100"
              :class="
                view.id == activeView.id && bench.showViewContent
                  ? 'border-orange-600 text-orange-600'
                  : 'hover:border-orange-100'
              "
              @click="toggleActiveView(view.id, true)"
            >
              <span class="sr-only">{{ view.label }}</span>
              <component :is="view.icon" class="h-6 w-6" aria-hidden="true" />
              <!-- Tooltip -->
              <span
                v-if="view.id != activeView.id || !bench.showViewContent"
                class="pointer-events-none absolute left-full top-3 z-30 rounded-sm bg-white px-1.5 text-xs opacity-0 ring-1 ring-orange-900 ring-opacity-[25%] transition duration-75 group-hover:opacity-100"
              >
                {{ view.label }}
              </span>
            </button>
          </div>
          <!-- Bottom of sidebar: feedback, help, settings popovers -->
          <component v-for="popover in sidebarPopovers" :key="popover.label" :is="popover.component">
            <template v-slot:button="{ open }">
              <PopoverButton
                class="group relative border-l-2 px-2 py-2.5 text-gray-600 outline-none hover:bg-orange-100 focus:ring-0"
                :class="open ? 'border-orange-600 text-orange-600' : 'hover:border-orange-100'"
              >
                <span class="sr-only">{{ popover.label }}</span>
                <component :is="popover.icon" class="h-6 w-6" aria-hidden="true" />
                <!-- Tooltip -->
                <span
                  v-if="!open"
                  class="pointer-events-none absolute left-full top-3 z-10 whitespace-nowrap rounded-sm bg-white px-1.5 text-xs opacity-0 ring-1 ring-orange-900 ring-opacity-[25%] transition-opacity duration-75 group-hover:opacity-100"
                >
                  {{ popover.label }}
                </span>
              </PopoverButton>
            </template>
          </component>
        </div>
        <!-- View content -->
        <!-- note: border-r is 3% less opacity because it looks darker (optical illusion) -->
        <div
          ref="viewContainerRef"
          class="relative h-full max-h-full max-w-full flex-1 border-r border-orange-900 border-opacity-[10%]"
          v-show="bench.showViewContent"
        >
          <!-- 36 == :GlobalHeaderHeight -->
          <component
            :is="activeView.view"
            :ref="(ref: any) => viewsRefs.registerRef(activeView.id, ref)"
            @show="bench.focusView(activeView.id)"
            @blur="bench.blurView(activeView.id)"
            :active="bench.activeViewId == activeView.id"
            :focused="bench.focusedViewId == activeView.id"
            :container-size="viewContainerSize"
            :project="projectFetched"
            class="scroll-hidden overflow-y-hidden"
            :style="{ width: '276px', height: windowHeight - 40 + 'px' }"
          />
        </div>
      </aside>
      <!-- Main editor area -->
      <main
        ref="mainContainerRef"
        v-show="projectLoaded"
        class="flex h-full w-full flex-1 divide-x divide-orange-900 divide-opacity-[12%] bg-gray-50"
      >
        <!-- Left editor group -->
        <div class="relative flex-1 flex-shrink-0">
          <div class="absolute left-0 top-0 h-full w-full overflow-hidden">
            <PanelGroup :group="bench.left" class="h-full w-full" />
          </div>
        </div>
        <!-- Right editor group -->
        <div class="relative flex-1 flex-shrink-0" v-if="bench.right.panels.length > 0">
          <div class="absolute left-0 top-0 h-full w-full overflow-hidden">
            <PanelGroup :group="bench.right" class="h-full w-full" />
          </div>
        </div>
      </main>
    </div>
    <GenericNotFound v-if="!projectLoading && !projectLoaded" class="pb-12" />
    <NotificationArea />
  </div>
</template>
