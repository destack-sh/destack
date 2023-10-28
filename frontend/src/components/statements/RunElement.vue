<script lang="ts" setup>
import CodeBlock from "@/components/basic/CodeBlock.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import RunTile from "@/components/tiles/RunTile.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { StatementType } from "@/gql/graphql";
import { useBenchState, type StatementAction, usePanelContext } from "@/state/bench";
import { TypeFlag, useCurrentModule } from "@/state/module";
import { useCurrentSessions } from "@/state/session";
import { IdentifierType, toPyIdentifier } from "@/utils/functools";
import { LinkIcon, PlayIcon, StopIcon, WindowIcon } from "@heroicons/vue/24/outline";
import { computed, ref, type Ref } from "vue";

const props = defineProps<Pick<StatementProps, "statement" | "focused" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const sessions = useCurrentSessions();
const bench = useBenchState();
const module = useCurrentModule();
const panel = usePanelContext();

const { currentRun, currentRunActive } = sessions.currentRunOf(props.statement);
const showingIntegration = ref(false);
const integrationPopoverRef: Ref<HTMLDivElement | null> = ref(null);
const popoverPin = pinAbsoluteElement(integrationPopoverRef, { pos: true, keepInView: true });

const integrationCode = computed(() => {
  if (!showingIntegration.value) return "";
  const language = "python";
  const nodePath = module.nodePathOf(props.statement.ck);
  const path = nodePath?.map((n) => toPyIdentifier(n.name ?? "", IdentifierType.PATH)).join(".");
  const token = "BENCH_ACCESS_TOKEN";
  const url = `api.bench.is/${module.path.value?.replace(".", "/")}/run`;

  return `\
import requests

# TODO set 'token' and 'inputs'
headers = {'Authorization': 'Bearer ${token}'}
response = requests.post(
  '${url}',
  headers=headers,
  json={
    'statement': '${path}',
    'inputs': inputs,
  }
)
response.raise_for_status()
outputs = response.json()['outputs']
`;
});

const actions = computed(() => {
  const actions: StatementAction[] = [];
  if (!props.statement.fields.some((f) => f.deletedAt == null && !(f.flags & TypeFlag.IS_OUTPUT))) {
    // can only run inline if no inputs
    actions.push({
      label: currentRunActive.value ? "Stop" : "Run",
      groupId: "run",
      disabled: !bench.canUse,
      hideInline: true, // already have on left-hand side
      icon: currentRunActive.value ? StopIcon : PlayIcon,
      action: () => {
        if (currentRunActive.value) {
          sessions.kill(currentRun.value);
        } else {
          emit("run");
        }
      },
    });
  }
  actions.push({
    groupId: "run",
    label: "Launch",
    icon: WindowIcon,
    action: () => {
      bench.openLaunchRun(props.statement, { group: panel.panel.value.group, opposite: true, focus: true });
    },
  });
  actions.push({
    groupId: "misc",
    label: "Integrate",
    hideInline: true,
    icon: LinkIcon,
    action: () => {
      showingIntegration.value = true;
    },
  });

  return actions;
});

defineExpose({
  focus: () => {
    // nothing?
  },
  blur: () => {
    // nothing?
  },
  actions,
});
</script>
<template>
  <div>
    <RunTile
      v-if="currentRun != null"
      ref="runTileRef"
      class="relative -mx-1 mb-0.5 w-full rounded-b-sm border border-orange-900/[12%] px-3 py-1.5 transition duration-150"
      :class="[props.statement.type == StatementType.Code ? 'border-t-0' : 'mt-1.5']"
      :project-id="(bench.projectId as string)"
      :project-version-id="(bench.projectVersionId as string)"
      :run="currentRun"
      :key="currentRun?.id"
      :view="props.statement.type == StatementType.Code ? (currentRun.errorNice != null ? 'error' : 'logs') : 'trace'"
      show-controls
      show-close
      @close="emit('hide', 'run')"
    />
    <!-- Prevent scroll and capture click outside -->
    <div
      v-if="showingIntegration"
      class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
      @click.stop="showingIntegration = false"
    />
    <!-- TODO @UX: centralize integration docs/help (panel?) -->
    <!-- Integration help popover -->
    <FadeTransition>
      <div
        v-if="showingIntegration"
        ref="integrationPopoverRef"
        class="z-50 flex w-[500px] max-w-full flex-col rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :class="[popoverPin.pinned.value ? '' : 'absolute right-0 top-0']"
        @keydown.escape.exact.prevent.stop="showingIntegration = false"
      >
        <!-- Header -->
        <div class="flex w-full flex-row items-baseline">
          <h5 class="text-sm font-bold text-gray-900">Integrate {{ props.statement.name ?? "???" }}</h5>
          <span class="ml-auto text-xs text-gray-500">Python</span>
        </div>
        <!-- Code -->
        <div class="whitespace-wrap mt-1 w-full">
          <CodeBlock language="python" :model-value="integrationCode" />
        </div>
      </div>
    </FadeTransition>
  </div>
</template>
