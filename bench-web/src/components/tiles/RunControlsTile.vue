<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import type { Run } from "@/gql/graphql";
import { LaunchRunPanel, useBenchState } from "@/state/bench";
import { ACTIVE_RUN_STATUSES, useCurrentSessions } from "@/state/session";
import { ForwardIcon, PlayIcon, StopIcon, WindowIcon } from "@heroicons/vue/24/solid";
import { computed, type Ref } from "vue";

const props = defineProps<{
  run?: Run;
  statement?: { id: string; ck: string };
  inputs?: any;
  hide?: ActionId[];
}>();
const emit = defineEmits<{
  (e: "run", run: Run): void;
  (e: "kill"): void;
  (e: "rerun", run: Run): void;
}>();

const bench = useBenchState();
const sessions = useCurrentSessions();

type ActionId = "run" | "stop" | "rerun" | "launch";
type Action = {
  id: ActionId;
  label: string;
  icon: any;
  highlight?: boolean;
  active?: boolean;
  disabled: boolean;
  action: () => void;
};
const runActive = computed(() => props.run != null && ACTIVE_RUN_STATUSES.includes(props.run.status));
const actions: Ref<Action[]> = computed(() => [
  {
    id: "run",
    label: "Run",
    icon: PlayIcon,
    highlight: true,
    active: runActive.value,
    disabled: !bench.canUse || runActive.value || props.statement == null,
    action: () => {
      if (props.statement == null) throw new Error("statement not set");
      const { run } = sessions.run(props.statement, { inputs: props.inputs ?? {}, keyed: true });
      emit("run", run);
    },
  },
  {
    id: "launch",
    label: "Launch",
    icon: WindowIcon,
    disabled: !bench.canUse || props.run == null || props.statement == null,
    action: () => {
      if (props.run == null || props.statement == null) throw new Error("run not set");
      const panel = bench.openLaunchRun(props.statement, { focus: true }) as LaunchRunPanel;
      panel.inputs = props.run.inputs;
    },
  },
  {
    id: "rerun",
    label: "Rerun",
    icon: ForwardIcon,
    disabled: !bench.canUse || props.run == null || props.statement == null,
    action: () => {
      if (props.run == null || props.statement == null) throw new Error("run not set");
      if (runActive.value) {
        sessions.kill(props.run);
        emit("kill");
      }
      const { run: newRun } = sessions.run(props.statement, { inputs: props.run.inputs ?? {}, keyed: true });
      emit("rerun", newRun);
    },
  },
  {
    id: "stop",
    label: "Stop",
    icon: StopIcon,
    active: props.run != null && sessions.isKilling(props.run),
    disabled: !bench.canUse || !runActive.value,
    action: () => {
      if (props.run == null) throw new Error("run not set");
      sessions.kill(props.run);
      emit("kill");
    },
  },
]);
</script>
<template>
  <div class="flex flex-row gap-1.5">
    <button
      v-for="action in actions.filter((a) => !props.hide?.includes(a.id))"
      :key="action.label"
      class="flex flex-row items-center rounded-sm border border-orange-900/[12%] px-2 py-1 shadow-sm"
      :class="[
        action.disabled
          ? 'focus:border-opacity-40'
          : action.highlight
          ? 'bg-orange-600 hover:bg-orange-500 focus:bg-orange-500'
          : 'bg-white hover:bg-orange-100 focus:bg-orange-100',
      ]"
      :disabled="action.disabled"
      @click.stop="action.action"
    >
      <BusySpinnerIcon v-if="action.active" class="mr-1 h-5 w-5 animate-spin" />
      <component
        v-else
        :is="action.icon"
        class="mr-1 h-5 w-5"
        :class="action.disabled ? 'text-gray-300' : action.highlight ? 'text-white' : 'text-gray-500'"
      />
      <span :class="action.disabled ? 'text-gray-500' : action.highlight ? 'text-white' : 'text-gray-900'">
        {{ action.label }}
      </span>
    </button>
  </div>
</template>
