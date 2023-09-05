<script lang="ts" setup>
import type { StatementEmit, StatementProps } from "@/components/statements";
import RunTile from "@/components/tiles/RunTile.vue";
import { StatementType } from "@/gql/graphql";
import { useBenchState, type StatementAction, usePanelContext } from "@/state/bench";
import { TypeFlag } from "@/state/module";
import { useCurrentSessions } from "@/state/session";
import { PlayIcon, WindowIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";

const props = defineProps<Pick<StatementProps, "statement" | "focused" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const sessions = useCurrentSessions();
const bench = useBenchState();
const panel = usePanelContext();

const { currentRun, currentRunActive } = sessions.currentRunOf(props.statement);

defineExpose({
  focus: () => {
    // nocheckin
  },
  blur: () => {
    // nothing
  },
  actions: computed(() => {
    const actions: StatementAction[] = [];
    if (!props.statement.fields.some((f) => f.deletedAt == null && !(f.flags & TypeFlag.IsOutput))) {
      // can only run inline if no inputs
      actions.push({
        label: "Run",
        groupId: "run",
        disabled: currentRunActive.value,
        icon: PlayIcon,
        action: () => {
          emit("run");
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

    return actions;
  }),
});
</script>
<template>
  <RunTile
    v-if="currentRun != null"
    ref="runTileRef"
    class="relative -mx-1 mb-0.5 w-full rounded-b-sm border border-orange-900 border-opacity-[12%] px-3 py-1.5 transition duration-150"
    :class="[props.statement.type == StatementType.Code ? 'border-t-0' : 'mt-1.5']"
    :project-id="(bench.projectId as string)"
    :project-version-id="(bench.projectVersionId as string)"
    :run="currentRun"
    :key="currentRun?.id"
    :view="props.statement.type == StatementType.Code ? (currentRun.errorNice != null ? 'error' : 'logs') : 'trace'"
    show-controls
  />
</template>
