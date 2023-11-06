<script lang="ts" setup>
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useAppearance } from "@/state/appearance";
import { usePanelContext, type Action } from "@/state/bench";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { ChevronRightIcon, EllipsisVerticalIcon, SparklesIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, watch, type Ref, shallowRef } from "vue";
import uFuzzy from "@leeoniya/ufuzzy";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useActiveScroll } from "@/composables/useScroll";

const props = defineProps<{
  actions: Action<any>[];
  thing: any;
  anchor: "left" | "right";
  hideSearch?: boolean;
  small?: boolean;
  allowFreeform?: boolean;
}>();
const emit = defineEmits<{
  (e: "mousedown", v: MouseEvent): void;
  (e: "mouseup", v: MouseEvent): void;
  (e: "click", v: MouseEvent): void;
  (e: "open"): void;
  (e: "close"): void;
  (e: "freeform", v: string): void;
}>();

const popoverPanelRef: Ref<HTMLDivElement | null> = ref(null);
const open: Ref<boolean> = ref(false);
const inputRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);
const { pinned: popoverPinned, fixed: popoverFixed } = pinAbsoluteElement(
  computed(() => popoverPanelRef.value),
  { pos: true, width: true, keepInView: true }
);
const panel = usePanelContext();
const isInRightThirdOfPanel = computed(() => {
  if (popoverFixed.value == null) return false;
  return popoverFixed.value.x + popoverFixed.value.width > panel.size.value.width * 0.66;
});

useActiveScroll(computed(() => popoverPanelRef.value));

const query = ref("");
const uf = new uFuzzy({ intraMode: 0 });
const filteredActions = computed(() => {
  const actions = props.actions.filter((a) => !a.hideInMenu);
  if (query.value.trim() == "") return actions;
  const [idxs, info, order] = uf.search(
    actions.map((a) => a.label),
    query.value,
    true
  );
  if (idxs && order) {
    return order.map((i) => actions[idxs[i]]);
  }
  return [];
});
const closed = ref(false);
const nestedComponent = shallowRef<{ component: InstanceType<any> | null; props: any } | null>(null);
const nestedActionIndex = ref<number | null>(null);

watch(open, () => {
  if (open.value) {
    // focus input when popover opens
    emit("open");
    nextTick(focus);
  } else {
    // clear input and closed when popover opens/closes
    emit("close");
    nextTick(blur);
  }
});

function close() {
  open.value = false;
  closed.value = true;
}

function focus() {
  inputRef.value?.$el.focus();
}

function blur() {
  query.value = "";
  closed.value = false;
  nestedComponent.value = null;
  nestedActionIndex.value = null;
}

function openComponent(action: Action<any>) {
  if (action.component == null) return;
  nestedComponent.value = action.component(props.thing);
  nestedActionIndex.value = props.actions.findIndex((a) => a.label == action.label);
}

function selectAction(action: Action<any> | "freeform", close: () => void) {
  if (action == "freeform") {
    emit("freeform", query.value);
    close();
  } else if (action.component != null) {
    openComponent(action);
  } else {
    doActionIfOpen(action);
    close();
  }
}

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
  focus,
  blur,
  show: () => (open.value = true),
  open,
});
</script>
<template>
  <div class="relative">
    <button
      class="z-20 block rounded-sm text-gray-900 hover:bg-orange-100 focus:bg-orange-100 focus:outline-none focus:ring-0"
      :class="[open ? 'bg-orange-100' : '']"
      @click="emit('click', $event), (open = true), (closed = false), $nextTick(() => inputRef?.$el.focus())"
      @mousedown.stop.prevent="emit('mousedown', $event)"
      @mouseup="emit('mouseup', $event)"
    >
      <slot :close="close" :open="open"><EllipsisVerticalIcon class="h-4 w-4" /></slot>
    </button>
    <!-- Prevent scroll and capture click outside -->
    <div v-if="open" class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="close" />
    <FadeTransition>
      <div
        v-if="open"
        ref="popoverPanelRef"
        as="div"
        class="z-50 flex flex-col gap-2 rounded-sm bg-white shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :class="[popoverPinned ? '' : 'absolute ' + anchor, small ? 'w-40 p-1' : 'w-72 p-2 ']"
        @keydown.escape.exact.prevent.stop="close"
      >
        <span ref="popoverOpenRef" class="hidden" />
        <!-- Input & actions -->
        <!-- note: we use closed to ensure action is only called once (since it's triggered by update model value and click) -->
        <Combobox as="div" :model-value="null" @update:model-value="(action: any) => selectAction(action, close)">
          <ComboboxInput
            v-if="!small && !hideSearch"
            as="input"
            ref="inputRef"
            class="w-full rounded-sm border border-orange-900/[12%] bg-orange-100 p-1 text-gray-900 outline-none ring-0 placeholder:text-gray-400 hover:bg-orange-100 focus:border-orange-900 focus:border-opacity-[12%] focus:ring-0"
            :class="{
              ...appearance.baseClass,
            }"
            @change="query = $event.target.value"
            :display-value="(el: any) => ''"
            :placeholder="allowFreeform ? 'Search or describe...' : 'Search actions...'"
            spellcheck="false"
            @keydown.enter.prevent.stop="close"
          />
          <ComboboxOptions
            class="scroll-hidden max-h-[220px] overflow-auto"
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
              @click.prevent.stop="selectAction(action, close)"
              @keydown.enter.prevent.stop="selectAction(action, close)"
              @keydown.right.prevent.stop="openComponent(action)"
              class="flex flex-row items-center justify-between"
              :class="[
                i > 0 && filteredActions[i - 1].groupId != action.groupId
                  ? ' border-t border-orange-900/[12%] ' + (small ? 'mt-0.5 pt-0.5' : 'mt-1 pt-1')
                  : '',
              ]"
            >
              <!-- Actual label -->
              <button
                class="flex w-full flex-row items-center gap-2.5 rounded-sm px-1 py-1 focus:outline-none"
                :class="[
                  active ? 'bg-orange-100' : '',
                  action.disabled || action.active ? 'cursor-not-allowed opacity-50' : '',
                ]"
              >
                <component :is="action.icon" class="h-4 w-4" />
                <span class="truncate text-gray-700">{{ action.label }}</span>
              </button>
              <!-- Keyboard shortcut or chevron for nested action components -->
              <div
                v-if="action.component != null"
                class="px-1 py-1.5 text-gray-400"
                :class="[active ? 'bg-orange-100' : '']"
              >
                <ChevronRightIcon class="h-4 w-4" />
              </div>
            </ComboboxOption>
            <!-- Freeform -->
            <ComboboxOption
              v-if="allowFreeform && query.length > 0"
              :value="'freeform'"
              :class="filteredActions.length > 0 ? 'mt-1 border-t border-orange-900/[12%] pt-1' : ''"
              v-slot="{ active }"
            >
              <button
                class="flex w-full flex-row items-center gap-2.5 px-1 py-1"
                :class="[active ? 'bg-orange-100' : '']"
              >
                <SparklesIcon class="h-4 w-4" />
                <span class="truncate text-gray-700">{{ query }}</span>
              </button>
            </ComboboxOption>
            <div v-if="filteredActions.length == 0 && !allowFreeform" class="pt-1 text-center">
              <span class="text-gray-400">No results</span>
            </div>
          </ComboboxOptions>
        </Combobox>
        <!-- Nested component -->
        <!-- TODO @UX: improve nested action popover component (smooth open/close on hover, better transitions, etc.) -->
        <FadeTransition>
          <div
            v-if="nestedComponent != null && popoverFixed != null"
            class="z-70 fixed rounded-sm bg-white shadow-md ring-1 ring-orange-900 ring-opacity-40"
            :style="{
              // offset is hardcoded because we don't know the width of the nested component :NestedActionComponentWidth
              left: isInRightThirdOfPanel
                ? popoverFixed.x - 160 + 'px'
                : popoverFixed.x + popoverFixed.width - 4 + 'px',
              top: popoverFixed.y + 'px',
            }"
          >
            <component :is="nestedComponent.component" v-bind="nestedComponent.props" @close="close" />
          </div>
        </FadeTransition>
      </div>
    </FadeTransition>
  </div>
</template>
