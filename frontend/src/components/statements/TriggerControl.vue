<script lang="ts" setup>
import TriggerInterface from "@/components/interfaces/TriggerInterface.vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useElementRefs } from "@/composables/useGrid";
import { useNow } from "@/composables/useNow";
import type { Trigger } from "@/gql/graphql";
import { ScheduleType, TriggerType } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import { TypeFlag, newNodeIdentity, useCurrentModule } from "@/state/module";
import { useOperations } from "@/state/operations";
import { useTriggers } from "@/state/statement";
import { TRIGGER_ICONS_SOLID, getTriggerSchedule, type TriggerSchedule, type TimeTrigger } from "@/state/trigger";
import { BoltIcon, Square2StackIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { PauseIcon } from "@heroicons/vue/24/solid";
import { DateTime } from "luxon";
import { computed, ref, toRef, type Ref, nextTick } from "vue";

const props = defineProps<Pick<StatementProps, "statement" | "readonly" | "focused" | "editing">>();
const emit = defineEmits<StatementEmit>();

const { triggers } = useTriggers(toRef(props, "statement"));
const module = useCurrentModule();
const ops = useOperations();

const editingTrigger = ref<string | null>(null);
const editablePopoverRef: Ref<HTMLDivElement | null> = ref(null);
const popoverPin = pinAbsoluteElement(editablePopoverRef, { pos: true, keepInView: true });
const actionRefs = useElementRefs();
const triggerInterfaceRef = ref<InstanceType<typeof TriggerInterface> | null>(null);
const triggerRefs = useElementRefs<HTMLButtonElement>();

const currentTrigger = computed(() => triggers.value.find((t) => t.id == editingTrigger.value));

function addNew() {
  const identity = newNodeIdentity(module.id.value, "Trigger");
  const newTrigger = {
    id: identity.id,
    ck: identity.ck,
    type: TriggerType.Time,
    active: false,
    timezone: "UTC",
    scheduleType: ScheduleType.Interval,
    interval: 60 * 60,
    cron: "0 */1 * * *",
    mapping: null,
  };
  ops.symbol.createTrigger(null, props.statement.id, newTrigger);
  editingTrigger.value = newTrigger.id;
  nextTick(() => triggerRefs.focus(newTrigger.id));
}

function editTrigger(trigger: Pick<Trigger, "id">) {
  editingTrigger.value = trigger.id;
}

function updateTrigger(trigger: Trigger) {
  const oldTrigger = triggers.value.find((t) => t.id == trigger.id);
  if (oldTrigger == null) return;
  ops.symbol.updateTrigger(null, oldTrigger, trigger);
}

function duplicateTrigger(trigger: Trigger) {
  const newTrigger = { ...trigger, ...newNodeIdentity(module.id.value, "Trigger") };
  ops.symbol.createTrigger(null, props.statement.id, newTrigger);
  editingTrigger.value = newTrigger.id;
}

function deleteTrigger(trigger: Trigger) {
  ops.symbol.softDeleteTrigger(null, props.statement.id, trigger);
  close();
}

function close() {
  editingTrigger.value = null;
}

const now = useNow(1000);
const triggerSchedules: Ref<(TriggerSchedule | null)[]> = computed(() =>
  triggers.value.map((t) => (t.type == TriggerType.Time ? getTriggerSchedule(t as TimeTrigger, now.value) : null))
);

const triggerActions = computed(() => [
  {
    label: "Duplicate trigger",
    icon: Square2StackIcon,
    action: duplicateTrigger,
  },
  {
    label: "Delete trigger",
    icon: TrashIcon,
    action: deleteTrigger,
  },
]);

defineExpose({
  focus: (focus: "first" | "last" = "first") => {
    if (focus == "first") {
      triggerRefs.focus(triggers.value[0]?.id);
    } else {
      triggerRefs.focus(triggers.value[triggers.value.length - 1]?.id);
    }
  },
  blur: () => {
    triggerRefs.refs.value.forEach((ref) => ref.blur?.());
    close();
  },
  actions: [
    {
      label: "Add trigger",
      groupId: "edit",
      icon: BoltIcon,
      disabled:
        props.readonly || props.statement.fields.some((f) => f.deletedAt == null && !(f.flags & TypeFlag.IS_OUTPUT)),
      action: () => addNew(),
    },
  ] as StatementAction[],
});
</script>
<template>
  <div class="group relative flex flex-row gap-1.5">
    <!-- Existing triggers -->
    <button
      v-for="(trigger, i) in triggers"
      :ref="(ref: any) => triggerRefs.registerRef(trigger.id, ref)"
      :key="trigger.id"
      class="group/trigger relative flex h-fit max-h-fit flex-row items-center rounded-xl px-1 ring-inset ring-rose-600/20 hover:bg-rose-200 hover:ring-1 focus:bg-rose-200 focus:outline-none focus:ring-rose-600/60"
      :class="[trigger.active ? 'text-orange-900' : 'text-gray-600']"
      @click="editTrigger(trigger)"
      @keydown.delete.exact.prevent="deleteTrigger(trigger)"
      @keydown.left.exact.prevent="i == 0 ? emit('navigateLeft') : triggerRefs.focus(triggers[i - 1]?.id)"
      @keydown.right.exact.prevent="
        i == triggers.length - 1 ? emit('navigateRight') : triggerRefs.focus(triggers[i + 1]?.id)
      "
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
    >
      <span class="mr-1 inline-flex flex-row">
        <component :is="TRIGGER_ICONS_SOLID[TriggerType.Time]" class="h-4 w-4" />
        <PauseIcon v-if="!trigger.active" class="h-4 w-4" />
      </span>
      <span class="max-w-[120px] truncate whitespace-nowrap text-sm font-semibold">
        {{ triggerSchedules[i]?.humanized ?? "???" }}
      </span>
      <!-- Full trigger + schedule on hover -->
      <div
        class="pointer-events-none absolute left-0 top-5 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-gray-700 opacity-0 transition duration-150 group-hover/trigger:opacity-100"
      >
        <span class="text-center font-bold"
          >{{ triggerSchedules[i]?.humanized ?? "Invalid schedule" }}
          <span v-if="!trigger.active" class="font-normal text-gray-400">(inactive)</span>
        </span>
        <!-- Occurrences -->
        <div
          v-if="
            triggerSchedules[i] != null &&
            triggerSchedules[i]?.lastOccurrence != null &&
            triggerSchedules[i]?.nextOccurrences != null
          "
          class="mt-1 flex flex-col"
        >
          <span
            v-for="(occurrence, offset) in [
              triggerSchedules[i]?.lastOccurrence,
              ...(triggerSchedules[i]?.nextOccurrences ?? []),
            ]"
            :key="offset"
            class="flex flex-row justify-between gap-2.5"
            :class="[offset == 1 ? (trigger.active ? 'text-orange-600' : 'text-gray-700') : 'text-gray-400']"
          >
            <span>{{ { 0: "last", 1: "next" }[offset] ?? "then" }}</span>
            <span>
              {{ occurrence?.setZone(trigger.timezone ?? "UTC").toLocaleString(DateTime.DATETIME_FULL_WITH_SECONDS) }}
            </span>
          </span>
        </div>
      </div>
    </button>
    <!-- Prevent scroll and capture click outside -->
    <div
      v-if="editing && currentTrigger != null"
      class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
      @click.stop="close()"
    />
    <!-- Edit popover -->
    <div
      v-if="editing && currentTrigger != null"
      ref="editablePopoverRef"
      class="z-50 flex w-96 flex-col rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      :class="popoverPin.pinned.value ? '' : 'absolute -left-2 -top-2'"
      @keydown.escape.exact.prevent.stop="close()"
    >
      <!-- Interface -->
      <TriggerInterface
        ref="triggerInterfaceRef"
        :readonly="false"
        :model-value="currentTrigger"
        @update:model-value="updateTrigger($event)"
        @navigate-down="actionRefs.focus(triggerActions[0].label)"
      >
        <template v-slot:actions>
          <div class="flex flex-row items-center gap-1 px-1">
            <button
              v-for="action in triggerActions"
              :key="action.label"
              class="p-0.5 text-gray-400 hover:bg-orange-100"
              @click="action.action(currentTrigger)"
            >
              <component :is="action.icon" class="h-4 w-4" />
            </button>
          </div>
        </template>
      </TriggerInterface>
    </div>
  </div>
</template>
