<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useElementRefs } from "@/components/cells/grid";
import SelectTypeCell from "@/components/cells/SelectTypeCell.vue";
import SimpleTypePreview from "@/components/cells/SimpleTypePreview.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { ANY_TYPE_NODE, getEnumColor, type SimpleType, type TypeAction } from "@/components/statement";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { TypeTag } from "@/gql/graphql";
import { syncProperty } from "@/utils/sync";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { AdjustmentsHorizontalIcon, Square2StackIcon } from "@heroicons/vue/24/outline";
import TrashIcon from "@heroicons/vue/24/outline/TrashIcon";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const props = defineProps<{
  modelValue?: SimpleType;
  readonly: boolean;
  inlined?: boolean;
  structrefOnly?: boolean;
  hideFlags?: boolean;
  extraActions?: TypeAction[];
  tupleName?: string;
  isEnum?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Pick<SimpleType, "name" | "tag" | "flags" | "reference">): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "deleteLeft"): void;
  (e: "deleteSelf"): void;
  (e: "duplicateSelf"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "focus", event: FocusEvent): void;
}>();

const tupleName = computed(() => props.tupleName ?? "field");
const value: Ref<SimpleType> = ref(props.modelValue ?? ANY_TYPE_NODE);
const name: Ref<string> = ref(props.modelValue?.name ?? "");

const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const nameRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const typeButtonRef: Ref<HTMLButtonElement | null> = ref(null);
const popoverButtonRef: Ref<InstanceType<typeof PopoverButton> | null> = ref(null);
const typePopoverButtonRef: Ref<InstanceType<typeof PopoverButton> | null> = ref(null);
const popoverOpenRef: Ref<HTMLSpanElement | null> = ref(null);
const actionRefs = useElementRefs();

// pin popover to the right
const popoverPanelRef: Ref<InstanceType<typeof PopoverPanel> | null> = ref(null);
const popoverPin = pinAbsoluteElement(
  computed(() => popoverPanelRef.value?.$el),
  { pos: true }
);
const typePopoverPanelRef = ref<InstanceType<typeof PopoverPanel> | null>(null);
const typePopoverPin = pinAbsoluteElement(
  computed(() => typePopoverPanelRef.value?.$el),
  { pos: true }
);

syncProperty({
  value: name,
  editing: computed(() => nameRef.value?.focused),
  read: () => (name.value = value.value.name ?? ""),
  write: () => {
    value.value = {
      ...value.value,
      name: name.value,
    };
    emit("update:modelValue", value.value);
  },
});

// sync model value into local value
watch(
  () => [props.modelValue],
  () => {
    value.value = props.modelValue ?? ANY_TYPE_NODE;
  }
);

const actions: Ref<TypeAction[]> = computed(() => {
  const actions = [];
  if (!props.readonly) {
    if (!props.isEnum) {
      actions.push({
        label: "Edit type",
        icon: AdjustmentsHorizontalIcon,
        keepOpen: true,
        action: () => typeButtonRef.value?.click(),
      });
    }
    actions.push({
      label: "Duplicate " + tupleName.value,
      icon: Square2StackIcon,
      action: () => emit("duplicateSelf"),
    });
    actions.push({
      label: "Delete " + tupleName.value,
      icon: TrashIcon,
      action: () => emit("deleteSelf"),
    });
  }
  if (props.extraActions != null) {
    actions.push(...props.extraActions);
  }
  return actions;
});

function open() {
  if (popoverOpenRef.value == null) {
    popoverButtonRef.value?.$el.click();
    if (!props.readonly) {
      nextTick(() => nameRef.value?.focus());
    }
    document.body.classList.add("overscroll-y-none");
  }
}

watch(popoverOpenRef, (open) => {
  if (open == null) {
    document.body.classList.remove("overscroll-y-none");
  }
});

function focus() {
  buttonRef.value?.focus();
}

function blur() {
  buttonRef.value?.blur();
}

defineExpose({
  editing: computed(() => popoverOpenRef.value != null),
  focus,
  blur,
});
</script>
<template>
  <Popover as="div" class="relative" v-slot="{ close }">
    <!-- Preview -->
    <button
      ref="buttonRef"
      tabindex="-1"
      @keydown.left.exact.prevent="emit('navigateLeft')"
      @keydown.right.exact.prevent="emit('navigateRight')"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
      @keydown.delete.exact="editing || emit('deleteSelf')"
      class="flex h-full w-full flex-col text-left outline-none"
      :class="[isEnum ? 'bg-gray-100 px-2' : '']"
      @click="open"
      @keydown.enter.exact.prevent="open"
    >
      <!-- :EnumStyle -->
      <!-- Inner div so we can keep the button at the right height without the items-center below centering everything vertically -->
      <div class="flex w-full flex-row items-center text-left">
        <svg
          v-if="isEnum"
          class="mr-1.5 h-1.5 w-1.5"
          :style="{ fill: getEnumColor(value) }"
          viewBox="0 0 6 6"
          aria-hidden="true"
        >
          <circle cx="3" cy="3" r="3" />
        </svg>
        <span
          class="mr-2 text-gray-900"
          :class="[inlined ? 'underline decoration-gray-400 decoration-dashed underline-offset-4' : '']"
          >{{ value.name }}</span
        >
        <SimpleTypePreview v-if="!isEnum" :type="value" :hide-icon="value.reference != null" />
      </div>
    </button>
    <!-- Hidden popover button to proxy the button to because I can't figure out key events on the popover button directly -->
    <PopoverButton ref="popoverButtonRef" @focus.prevent="focus" class="hidden" />
    <!-- Prevent scroll and capture click outside -->
    <div
      v-if="popoverOpenRef != null"
      class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
      @click.stop="close"
    />
    <!-- Edit popover -->
    <FadeTransition>
      <!-- Popover position is pinned -->
      <PopoverPanel
        ref="popoverPanelRef"
        class="z-50 flex w-64 flex-col gap-2 rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :class="popoverPin.pinned.value ? '' : 'absolute -left-2 -top-2 '"
      >
        <span ref="popoverOpenRef" class="hidden" />
        <!-- Name & type -->
        <div class="flex flex-row items-center justify-between gap-2">
          <!-- Name -->
          <EditableSpan
            ref="nameRef"
            v-model="name"
            :readonly="readonly"
            class="w-full rounded-sm border border-orange-900 border-opacity-[12%] p-1 text-gray-900 focus:bg-orange-100"
            @navigate-right="typeButtonRef?.focus()"
            @navigate-down="actionRefs.focus(actions[0].label)"
            @enter="
              close();
              buttonRef?.focus();
            "
          />
          <!-- Type popover -->
          <Popover v-if="!isEnum" as="div" class="relative" v-slot="{ close }">
            <button
              ref="typeButtonRef"
              :disabled="readonly"
              class="rounded-sm border border-orange-900 border-opacity-[12%] p-1 text-gray-900 focus:bg-orange-100 focus:outline-none focus:ring-0"
              :class="readonly ? '' : 'hover:bg-orange-100'"
              @keydown.left="nameRef?.focus()"
              @keydown.down="actionRefs.focus(actions[0].label)"
              @click="typePopoverButtonRef?.$el.click()"
              @keydown.enter.stop.prevent="typePopoverButtonRef?.$el.click()"
            >
              <!-- No idea why but this needs to be set absolutely or the icons are too high -->
              <SimpleTypePreview class="absolute top-0.5" :type="value" hide-reference />
            </button>
            <PopoverButton ref="typePopoverButtonRef" @focus.prevent="typeButtonRef?.focus" class="hidden" />
            <!-- Popover position is also pinned -->
            <PopoverPanel
              ref="typePopoverPanelRef"
              class="z-10 flex w-64 flex-col gap-2 rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
              :class="typePopoverPin.pinned.value ? '' : 'absolute -left-1 -top-10'"
              unmount
            >
              <SelectTypeCell
                :model-value="value"
                @update:model-value="emit('update:modelValue', $event)"
                @escape="
                  close();
                  typeButtonRef?.focus();
                "
              />
            </PopoverPanel>
          </Popover>
          <!-- Enum color (not yet editable) -->
          <div
            v-else
            ref="typeButtonRef"
            class="rounded-sm border border-orange-900 border-opacity-[12%] p-2 hover:bg-orange-100 focus:bg-orange-100 focus:outline-none focus:ring-0"
          >
            <svg class="h-3 w-3" :style="{ fill: getEnumColor(value) }" viewBox="0 0 6 6" aria-hidden="true">
              <rect x="0" y="0" width="6" height="6" />
            </svg>
          </div>
        </div>
        <!-- Actions -->
        <div class="mt-0.5 flex flex-col gap-0.5" v-if="actions.length > 0">
          <button
            v-for="(action, i) in actions"
            :ref="(el: any) => actionRefs.registerRef(action.label, el)"
            :key="action.label"
            class="flex w-full flex-row items-center gap-2.5 rounded-sm px-1 py-1 hover:bg-orange-100 focus:bg-orange-100 focus:outline-none"
            @click="
              action.action(value);
              action.keepOpen || close();
            "
            @keydown.enter.prevent.stop="
              action.action(value);
              action.keepOpen || close();
            "
            @keydown.up.exact.stop.prevent="i == 0 ? nameRef?.focus() : actionRefs.focus(actions[i - 1].label)"
            @keydown.down.exact.stop.prevent="i == actions.length - 1 ? null : actionRefs.focus(actions[i + 1].label)"
          >
            <component :is="action.icon" class="h-4 w-4 text-gray-500" />
            <span class="text-gray-700">{{ action.label }}</span>
          </button>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
