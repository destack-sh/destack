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
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useActiveScroll } from "@/composables/useScroll";

const props = defineProps<{
  actions: Action<any>[];
  thing: any;
  anchor: "left" | "right";
  hideSearch?: boolean;
  small?: boolean;
}>();
const emit = defineEmits<{
  (e: "mousedown", v: MouseEvent): void;
  (e: "mouseup", v: MouseEvent): void;
  (e: "click", v: MouseEvent): void;
  (e: "open"): void;
  (e: "close"): void;
}>();

const popoverPanelRef: Ref<InstanceType<typeof PopoverPanel> | null> = ref(null);
const popoverButtonRef: Ref<InstanceType<typeof PopoverButton> | null> = ref(null);
const popoverOpenRef: Ref<HTMLElement | null> = ref(null);
const inputRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);
const popoverPin = pinAbsoluteElement(
  computed(() => popoverPanelRef.value?.$el),
  { pos: true, width: true, keepInView: true }
);

useActiveScroll(computed(() => popoverPanelRef.value?.$el));

const query = ref("");
const uf = new uFuzzy({ intraMode: 0 });
const filteredActions = computed(() => {
  const actions = props.actions.filter((a) => !a.hideInMenu);
  if (query.value.trim() == "") return actions;
  const [idxs] = uf.search(
    actions.map((a) => a.label),
    query.value
  );
  return idxs?.map((idx) => actions[idx]) ?? [];
});
const closed = ref(false);

// focus input when popover opens
watch(popoverOpenRef, () => {
  emit("open");
  nextTick(() => inputRef.value?.$el.focus());
});
// clear input and closed when popover opens/closes
watch(popoverOpenRef, () => {
  emit("close");
  nextTick(() => ((query.value = ""), (closed.value = false)));
});

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
      @mousedown.stop.prevent="emit('mousedown', $event)"
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
    <FadeTransition>
      <PopoverPanel
        ref="popoverPanelRef"
        as="div"
        class="z-50 flex flex-col gap-2 rounded-sm bg-white shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :class="[popoverPin.pinned.value ? '' : 'absolute ' + anchor, small ? 'w-40 p-1' : 'w-64 p-2 ']"
        unmount
      >
        <span ref="popoverOpenRef" class="hidden" />
        <!-- Input & actions -->
        <!-- note: we use closed to ensure action is only called once (since it's triggered by update model value and click) -->
        <Combobox as="div" :model-value="null" @update:model-value="(action: any) => (doActionIfOpen(action), close())">
          <ComboboxInput
            v-if="!small && !hideSearch"
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
            class="max-h-[220px] overflow-auto"
            static
            :class="{
              'font-mono': appearance.fontMono,
              'mt-1 text-sm': !small && !hideSearch,
              'text-xs': small,
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
                  ? ' border-t border-orange-900 border-opacity-[12%] ' + (small ? 'mt-0.5 pt-0.5' : 'mt-1 pt-1')
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
            <div v-if="filteredActions.length == 0" class="pt-1 text-center">
              <span class="text-gray-400">No results</span>
            </div>
          </ComboboxOptions>
        </Combobox>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
