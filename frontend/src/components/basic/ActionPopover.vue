<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useAppearance } from "@/state/appearance";
import type { Action } from "@/state/bench";
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

const props = defineProps<{
  actions: Action<any>[];
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

// focus input when popover opens
watch(popoverOpenRef, () => nextTick(() => inputRef.value?.$el.focus()));

const query = ref("");
const filteredActions = computed(() =>
  props.actions.filter((action: { label: string }) => action.label.toLowerCase().includes(query.value.toLowerCase()))
);
const closed = ref(true);

const anchor = computed(
  () =>
    ({
      left: "left-0",
      right: "right-0",
    }[props.anchor])
);
const appearance = useAppearance();
</script>
<template>
  <Popover as="div" class="relative" v-slot="{ close, open }">
    <!-- Button proxy so we can handle drag events -->
    <button
      class="z-20 rounded-sm p-0.5 text-gray-900 hover:bg-orange-100 focus:bg-orange-100 focus:outline-none focus:ring-0"
      :class="[open ? 'bg-orange-100' : '']"
      @click="
        emit('click', $event), popoverButtonRef?.$el.click(), (closed = false), $nextTick(() => inputRef?.$el.focus())
      "
      @mousedown="emit('mousedown', $event)"
      @mouseup="emit('mouseup', $event)"
    >
      <slot :close="close" :open="open">
        <EllipsisVerticalIcon class="h-4 w-4" />
      </slot>
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
        class="z-50 flex w-64 flex-col gap-2 rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :class="[popoverPin.pinned.value ? '' : 'absolute ' + anchor]"
        unmount
      >
        <span ref="popoverOpenRef" class="hidden" />
        <!-- Input & actions -->
        <!-- note: we use closed to ensure action is only called once (since it's triggered by update model value and click) -->
        <Combobox
          as="div"
          :model-value="null"
          @update:model-value="(action: any) => (closed || (action.action(thing), closed=true, close()))"
        >
          <ComboboxInput
            as="input"
            ref="inputRef"
            class="w-full rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 p-1 text-gray-900 outline-none ring-0 placeholder:text-gray-400 hover:bg-orange-100 focus:border-orange-900 focus:border-opacity-[12%] focus:ring-0"
            :class="{
              ...appearance.baseClass,
            }"
            @change="query = $event.target.value"
            :display-value="(el: any) => null"
            placeholder="Search actions..."
            spellcheck="false"
            @keydown.enter.prevent.stop="close"
          />
          <ComboboxOptions
            class="mt-1 max-h-48 w-60 overflow-auto"
            static
            :class="{
              'font-mono': appearance.fontMono,
              'text-sm': appearance.textSmall,
              'text-md': !appearance.textSmall,
            }"
          >
            <!-- Options -->
            <ComboboxOption
              v-for="action in filteredActions"
              :key="action.label"
              :value="action"
              :disabled="action.disabled"
              v-slot="{ active }"
              @click.prevent.stop="closed || (action.action(thing), (closed = true), close())"
            >
              <button
                class="flex w-full flex-row items-center gap-2.5 rounded-sm px-1 py-1 focus:outline-none"
                :class="[active ? 'bg-orange-100' : '', action.disabled ? 'opacity-50' : '']"
              >
                <component :is="action.icon" class="h-4 w-4" />
                <span class="text-gray-700">{{ action.label }}</span>
              </button>
            </ComboboxOption>
          </ComboboxOptions>
        </Combobox>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
