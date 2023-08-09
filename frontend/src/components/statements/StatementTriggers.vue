<script lang="ts" setup>
import TriggerInterface from "@/components/interfaces/TriggerInterface.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useElementRefs } from "@/composables/useGrid";
import type { Trigger } from "@/gql/graphql";
import { ScheduleType, TriggerType } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { newTriggerId } from "@/state/operations/statement";
import { useStatementContext } from "@/state/statement";
import { TRIGGER_ICONS_SOLID } from "@/state/trigger";
import { BoltIcon, Square2StackIcon, TrashIcon } from "@heroicons/vue/24/outline";
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

function renderTrigger(trigger: Trigger): string {
  return "every 1h"; // nocheckin
}

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
      v-for="trigger in context.triggers.value"
      :key="trigger.id"
      class="flex flex-row items-center rounded-xl bg-orange-100 px-1.5 text-orange-900 ring-1 ring-inset ring-orange-600/20 hover:bg-orange-200"
      :class="[trigger.active ? 'ring-solid' : 'ring-']"
      @click="editTrigger(trigger)"
    >
      <component :is="TRIGGER_ICONS_SOLID[TriggerType.Time]" class="mr-1 h-4 w-4" />
      <span class="text-sm">{{ renderTrigger(trigger) }}</span>
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
      />
      <!-- Actions -->
      <div class="mt-1.5 flex flex-col border-t border-orange-900 border-opacity-[12%] pt-1.5">
        <button
          v-for="(action, i) in actions"
          :key="action.label"
          class="flex w-full flex-row items-center gap-2.5 rounded-sm px-1 py-1 hover:bg-orange-100 focus:bg-orange-100 focus:outline-none"
          @click.prevent.stop="action.action(currentTrigger as Trigger)"
          @keydown.enter.prevent.stop="action.action(currentTrigger as Trigger)"
          @keydown.up.exact.stop.prevent="
            i == 0 ? triggerInterfaceRef?.focus() : actionRefs.focus(actions[i - 1].label)
          "
          @keydown.down.exact.stop.prevent="i == actions.length - 1 ? null : actionRefs.focus(actions[i + 1].label)"
        >
          <component :is="action.icon" class="h-4 w-4 text-gray-500" />
          <span class="text-gray-700">{{ action.label }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
