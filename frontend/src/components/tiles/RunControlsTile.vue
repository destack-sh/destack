<script lang="ts" setup>
import type { Run } from "@/gql/graphql";
import { ACTIVE_RUN_STATUSES, useCurrentSessions } from "@/state/session";
import { ForwardIcon, PlayIcon, StopIcon } from "@heroicons/vue/24/solid";
import { computed, type Ref } from "vue";

const props = defineProps<{
  run?: Run;
  runnable?: { id: string; ck: string };
  inputs?: any;
  hideRun?: boolean;
}>();
const emit = defineEmits<{
  (e: "run", run: Run): void;
  (e: "kill"): void;
  (e: "rerun", run: Run): void;
}>();

const sessions = useCurrentSessions();

type Action = {
  label: string;
  icon: any;
  disabled: boolean;
  hidden?: boolean;
  action: () => void;
};
const runActive = computed(() => props.run != null && ACTIVE_RUN_STATUSES.includes(props.run.status));
const actions: Ref<Action[]> = computed(() => [
  {
    label: "Run",
    icon: PlayIcon,
    disabled: runActive.value || props.runnable == null,
    hidden: props.hideRun,
    action: () => {
      if (props.runnable == null) throw new Error("runnable not set");
      const { run } = sessions.run(props.runnable, { inputs: props.inputs ?? {}, keyed: true });
      emit("run", run);
    },
  },
  {
    label: "Stop",
    icon: StopIcon,
    disabled: !runActive.value,
    action: () => {
      if (props.run == null) throw new Error("run not set");
      sessions.cancel(props.run);
      emit("kill");
    },
  },
  {
    label: "Rerun",
    icon: ForwardIcon,
    disabled: props.run == null || props.runnable == null,
    action: () => {
      if (props.run == null || props.runnable == null) throw new Error("run not set");
      if (runActive.value) {
        sessions.cancel(props.run);
        emit("kill");
      }
      const { run: newRun } = sessions.run(props.runnable, { inputs: props.run.inputs ?? {}, keyed: true });
      emit("rerun", newRun);
    },
  },
]);
</script>
<template>
  <div class="flex flex-row gap-1.5">
    <button
      v-for="action in actions.filter((a) => !a.hidden)"
      :key="action.label"
      class="flex flex-row items-center rounded-sm border border-orange-900 border-opacity-[12%] bg-white px-2 py-1 shadow-sm"
      :class="[action.disabled ? 'focus:border-opacity-40' : 'hover:bg-orange-100 focus:bg-orange-100']"
      :disabled="action.disabled"
      @click="action.action"
    >
      <component
        :is="action.icon"
        class="mr-1 h-5 w-5"
        :class="[action.disabled ? 'text-gray-300' : 'text-gray-500']"
      />
      <span :class="[action.disabled ? 'text-gray-500' : 'text-gray-900']">{{ action.label }}</span>
    </button>
  </div>
</template>
