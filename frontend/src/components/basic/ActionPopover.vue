<script lang="ts" setup>
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useAppearance } from "@/state/appearance";
import type { Action, ActionGroup } from "@/state/bench";
import {
  Combobox,
  ComboboxInput,
  ComboboxOption,
  ComboboxOptions,
  Popover,
  PopoverButton,
  PopoverPanel,
} from "@headlessui/vue";
import { EllipsisVerticalIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, watch, type Ref } from "vue";
import uFuzzy from "@leeoniya/ufuzzy";

const props = defineProps<{
  actions: Action<any>[];
  groups?: ActionGroup[];
  thing: any;
  anchor: "left" | "right";
}>();
const emit = defineEmits<{
  (e: "mousedown", v: MouseEvent): void;
  (e: "mouseup", v: MouseEvent): void;
  (e: "click", v: MouseEvent): void;
}>();

const popoverPanelRef: Ref<InstanceType<typeof PopoverPanel> | null> = ref(null);
const popoverButtonRef: Ref<InstanceType<typeof PopoverButton> | null> = ref(null);
const popoverOpenRef: Ref<HTMLElement | null> = ref(null);
const inputRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);
const popoverPin = pinAbsoluteElement(
  computed(() => popoverPanelRef.value?.$el),
  { pos: true, width: true, keepInView: true }
);

const query = ref("");
const uf = new uFuzzy({ intraMode: 0 });
const filteredActions = computed(() => {
  if (query.value.trim() == "") return props.actions;
  const [idxs] = uf.search(
    props.actions.map((a) => a.label),
    query.value
  );
  return idxs?.map((idx) => props.actions[idx]) ?? [];
});
const closed = ref(false);

// focus input when popover opens
watch(popoverOpenRef, () => nextTick(() => inputRef.value?.$el.focus()));
// clear input and closed when popover opens/closes
watch(popoverOpenRef, () => nextTick(() => ((query.value = ""), (closed.value = false))));

function doActionIfOpen(action: Action<any>) {
  if (!closed.value) {
    action.action(props.thing);
  }
  closed.value = true;
}

const anchor = computed(
  () =>
    ({
      left: "left-0",
      right: "right-0",
    }[props.anchor])
);
const appearance = useAppearance();

defineExpose({
  show: () => popoverButtonRef.value?.$el.click(),
  open: computed(() => popoverOpenRef.value != null),
});
</script>
<template>
  <Popover as="div" class="relative" v-slot="{ close, open }">
    <!-- Button proxy so we can handle drag events -->
    <button
      class="z-20 block rounded-sm text-gray-900 hover:bg-orange-100 focus:bg-orange-100 focus:outline-none focus:ring-0"
      :class="[open ? 'bg-orange-100' : '']"
      @click="
        emit('click', $event), popoverButtonRef?.$el.click(), (closed = false), $nextTick(() => inputRef?.$el.focus())
      "
      @mousedown="emit('mousedown', $event)"
      @mouseup="emit('mouseup', $event)"
    >
      <slot :close="close" :open="open"><EllipsisVerticalIcon class="h-4 w-4" /></slot>
    </button>
    <PopoverButton ref="popoverButtonRef" class="hidden" />
    <!-- Prevent scroll and capture click outside -->
    <div
      v-if="popoverOpenRef != null"
      class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
      @click.stop="close"
    />
    <PopoverPanel
      ref="popoverPanelRef"
      as="div"
      class="z-50 flex w-64 flex-col gap-2 rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      :class="[popoverPin.pinned.value ? '' : 'absolute ' + anchor]"
      unmount
    >
      <span ref="popoverOpenRef" class="hidden" />
      <!-- Input & actions -->
      <!-- note: we use closed to ensure action is only called once (since it's triggered by update model value and click) -->
      <Combobox as="div" :model-value="null" @update:model-value="(action: any) => (doActionIfOpen(action), close())">
        <ComboboxInput
          as="input"
          ref="inputRef"
          class="w-full rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 p-1 text-gray-900 outline-none ring-0 placeholder:text-gray-400 hover:bg-orange-100 focus:border-orange-900 focus:border-opacity-[12%] focus:ring-0"
          :class="{
            ...appearance.baseClass,
          }"
          @change="query = $event.target.value"
          :display-value="(el: any) => ''"
          placeholder="Search actions..."
          spellcheck="false"
          @keydown.enter.prevent.stop="close"
        />
        <ComboboxOptions
          class="scroll-hidden mt-1 max-h-[220px] w-60 overflow-auto"
          static
          :class="{
            'font-mono': appearance.fontMono,
            'text-sm': appearance.textSmall,
            'text-md': !appearance.textSmall,
          }"
        >
          <!-- Options -->
          <ComboboxOption
            v-for="(action, i) in filteredActions"
            :key="action.label"
            :value="action"
            :disabled="action.disabled || action.active"
            v-slot="{ active }"
            @click.prevent.stop="doActionIfOpen(action), close()"
            :class="[
              i > 0 && filteredActions[i - 1].groupId != action.groupId
                ? 'mt-1 border-t border-orange-900 border-opacity-[12%] pt-1'
                : '',
            ]"
          >
            <button
              class="flex w-full flex-row items-center gap-2.5 rounded-sm px-1 py-1 focus:outline-none"
              :class="[
                active ? 'bg-orange-100' : '',
                action.disabled || action.active ? 'cursor-not-allowed opacity-50' : '',
              ]"
            >
              <component :is="action.icon" class="h-4 w-4" />
              <span class="text-gray-700">{{ action.label }}</span>
            </button>
          </ComboboxOption>
        </ComboboxOptions>
      </Combobox>
    </PopoverPanel>
  </Popover>
</template>
