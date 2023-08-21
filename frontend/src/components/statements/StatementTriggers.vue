<script lang="ts" setup>
import TriggerInterface from "@/components/interfaces/TriggerInterface.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useElementRefs } from "@/composables/useGrid";
import { useNow } from "@/composables/useNow";
import type { Trigger } from "@/gql/graphql";
import { ScheduleType, TriggerType } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { newTriggerId } from "@/state/operations/statement";
import { useStatementContext } from "@/state/statement";
import { TRIGGER_ICONS_SOLID, getTriggerSchedule, type TriggerSchedule, type TimeTrigger } from "@/state/trigger";
import { BoltIcon, Square2StackIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { PauseIcon } from "@heroicons/vue/24/solid";
import { DateTime } from "luxon";
import { computed, ref, type Ref } from "vue";

const context = useStatementContext();
const ops = useOperations();

const editing = ref<string | null>(null);
const editablePopoverRef: Ref<HTMLDivElement | null> = ref(null);
const popoverPin = pinAbsoluteElement(editablePopoverRef, { pos: true, keepInView: true });
const actionRefs = useElementRefs();
const triggerInterfaceRef = ref<InstanceType<typeof TriggerInterface> | null>(null);

const currentTrigger = computed(() => context.triggers.value.find((t) => t.id == editing.value));

function addNew() {
  const newTrigger = {
    id: newTriggerId(),
    type: TriggerType.Time,
    active: false,
    timezone: "UTC",
    scheduleType: ScheduleType.Interval,
    interval: 60 * 60,
    cron: "0 */1 * * *",
    mapping: null,
  };
  ops.symbol.createTrigger(null, context.statement.value.id, newTrigger);
  editing.value = newTrigger.id;
}

function editTrigger(trigger: Pick<Trigger, "id">) {
  editing.value = trigger.id;
}

function updateTrigger(trigger: Trigger) {
  const oldTrigger = context.triggers.value.find((t) => t.id == trigger.id);
  if (oldTrigger == null) return;
  ops.symbol.updateTrigger(null, oldTrigger, trigger);
}

function duplicateTrigger(trigger: Trigger) {
  const newTrigger = { ...trigger, id: newTriggerId() };
  ops.symbol.createTrigger(null, context.statement.value.id, newTrigger);
  editing.value = newTrigger.id;
}

function deleteTrigger(trigger: Trigger) {
  console.log("delete trigger", trigger);
  ops.symbol.softDeleteTrigger(null, context.statement.value.id, trigger);
  close();
}

function close() {
  editing.value = null;
}

const now = useNow(1000);
const triggerSchedules: Ref<(TriggerSchedule | null)[]> = computed(() =>
  context.triggers.value.map((t) =>
    t.type == TriggerType.Time ? getTriggerSchedule(t as TimeTrigger, now.value) : null
  )
);

const actions = computed(() => [
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
</script>
<template>
  <div class="group relative flex flex-row gap-1.5">
    <!-- Existing triggers -->
    <button
      v-for="(trigger, i) in context.triggers.value"
      :key="trigger.id"
      class="group/trigger relative flex flex-row items-center rounded-xl bg-orange-100 px-1.5 ring-1 ring-inset ring-orange-600/20 hover:bg-orange-200"
      :class="[trigger.active ? 'text-orange-900' : 'text-gray-600']"
      @click="editTrigger(trigger)"
    >
      <span class="mr-1 inline-flex flex-row">
        <component :is="TRIGGER_ICONS_SOLID[TriggerType.Time]" class="h-4 w-4" />
        <PauseIcon v-if="!trigger.active" class="h-4 w-4" />
      </span>
      <span class="max-w-[120px] truncate whitespace-nowrap text-sm">
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
    <!-- Add trigger button -->
    <button
      v-if="!context.readonly.value"
      class="group/add flex flex-row rounded-xl border-gray-600 border-opacity-25 px-1 py-0 text-gray-400 hover:bg-orange-100 hover:text-gray-700 group-hover/add:ring-1"
      :class="
        context.focused.value
          ? ''
          : 'opacity-0 transition-opacity duration-150 group-hover/statement:opacity-100 group-hover:opacity-100'
      "
      @click="addNew"
    >
      <BoltIcon class="mt-0.5 h-4 w-4" />
    </button>
    <!-- Prevent scroll and capture click outside -->
    <div v-if="editing" class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="close()" />
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
        @navigate-down="actionRefs.focus(actions[0].label)"
      >
        <template v-slot:actions>
          <div class="flex flex-row items-center gap-1 px-1">
            <button
              v-for="action in actions"
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
