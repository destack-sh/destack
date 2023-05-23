<script lang="ts" setup>
import { useElementRefs } from "@/components/cells/grid";
import SelectTypeCell from "@/components/cells/SelectTypeCell.vue";
import SimpleTypePreview from "@/components/cells/SimpleTypePreview.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { ANY_TYPE_NODE, getEnumColor, type SimpleType } from "@/components/statement";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useElementSize } from "@/composables/useSize";
import { setDragData, useRelativeDropZone } from "@/utils/drop";
import { syncProperty } from "@/utils/sync";
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
  orientation?: "horizontal" | "vertical";
  statementId?: string;
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
  (e: "drop", p: "above" | "below" | "left" | "right", v: Dragged): void;
}>();

const tupleName = computed(() => props.tupleName ?? "field");
const value: Ref<SimpleType> = ref(props.modelValue ?? ANY_TYPE_NODE);
const name: Ref<string> = ref(props.modelValue?.name ?? "");
const editing = ref(false);
const editingType = ref(false);

const containerRef: Ref<HTMLDivElement | null> = ref(null);
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const previewRef: Ref<HTMLDivElement | null> = ref(null);
const nameRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const typeButtonRef: Ref<HTMLButtonElement | null> = ref(null);
const actionRefs = useElementRefs();
const previewSize = useElementSize(previewRef);

// pin popover to the right
const editablePopoverRef: Ref<HTMLDivElement | null> = ref(null);
const popoverPin = pinAbsoluteElement(editablePopoverRef, { pos: true, keepInView: true });
const typeEditablePopoverRef = ref<HTMLDivElement | null>(null);
const typePopoverPin = pinAbsoluteElement(typeEditablePopoverRef, { pos: true, keepInView: true });

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

// actions

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

// drag & drop

const {
  isOverDropZone: dragOver,
  inLeftHalf: dragInLeftHalf,
  inRightHalf: dragInRightHalf,
  inTopHalf: dragInTopHalf,
  inBottomHalf: dragInBottomHalf,
} = useRelativeDropZone(containerRef, ["Type"], onDrop);

// TODO @UX: ensure type tuple drag start works even if the containing statement is not focused
//  no idea how this relates yet.
function onDragStart(e: DragEvent) {
  setDragData(e, { type: "Type", id: props.modelValue?.id, statementId: props.statementId });
  e.dataTransfer?.setDragImage(buttonRef.value as HTMLElement, 20, 20);
}

function onDrop(thing: File[] | { type: string; id: string } | null) {
  if (!Array.isArray(thing) && thing?.type == "Type") {
    const position =
      props.orientation == "horizontal"
        ? dragInLeftHalf.value
          ? "left"
          : "right"
        : dragInTopHalf.value
        ? "above"
        : "below";
    emit("drop", position, thing);
  }
}

function open() {
  if (!editing.value) {
    editing.value = true;
    if (!props.readonly) {
      nextTick(() => nameRef.value?.focus());
    }
  }
}

function focus() {
  buttonRef.value?.focus();
}

function blur() {
  buttonRef.value?.blur();
}

function close() {
  editing.value = false;
  editingType.value = false;
  nextTick(focus);
}

defineExpose({
  editing,
  focus,
  blur,
  previewSize,
});
</script>
<template>
  <div ref="containerRef" class="relative" :class="[dragOver ? 'bg-orange-100' : '']">
    <!-- :DragStyle -->
    <div
      v-if="!readonly && orientation == 'horizontal'"
      class="absolute -left-0.5 top-0 h-full w-1 bg-orange-300 transition"
      :class="dragOver && dragInLeftHalf ? 'opacity-100' : 'opacity-0'"
    />
    <div
      v-if="!readonly && orientation == 'horizontal'"
      class="absolute -right-0.5 top-0 h-full w-1 bg-orange-300 transition"
      :class="dragOver && dragInRightHalf ? 'opacity-100' : 'opacity-0'"
    />
    <div
      v-if="!readonly && orientation == 'vertical'"
      class="absolute -top-0.5 left-0 h-1 w-full bg-orange-300 transition"
      :class="dragOver && dragInTopHalf ? 'opacity-100' : 'opacity-0'"
    />
    <div
      v-if="!readonly && orientation == 'vertical'"
      class="absolute -bottom-0.5 left-0 h-1 w-full bg-orange-300 transition"
      :class="dragOver && dragInBottomHalf ? 'opacity-100' : 'opacity-0'"
    />
    <!-- Preview -->
    <button
      ref="buttonRef"
      tabindex="-1"
      class="relative flex h-full w-full flex-col text-left outline-none"
      :class="[isEnum ? 'bg-gray-100 px-2' : '']"
      @keydown.left.exact.prevent="emit('navigateLeft')"
      @keydown.right.exact.prevent="emit('navigateRight')"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
      @keydown.delete.exact="editing || emit('deleteSelf')"
      @keydown.enter.exact.prevent="open"
      @click.stop="open"
      @dragstart.stop="onDragStart"
      @mousedown="buttonRef?.setAttribute('draggable', 'true')"
      @mouseup="buttonRef?.setAttribute('draggable', 'false')"
    >
      <!-- :EnumStyle -->
      <!-- Inner div so we can keep the button at the right height without the items-center below centering everything vertically -->
      <div ref="previewRef" class="flex flex-row items-center text-left">
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
    <!-- Prevent scroll and capture click outside -->
    <div v-if="editing" class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="close" />
    <!-- Edit popover -->
    <!-- Popover position is pinned -->
    <div
      v-if="editing"
      ref="editablePopoverRef"
      class="z-50 flex w-64 flex-col gap-2 rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      :class="popoverPin.pinned.value ? '' : 'absolute -left-2 -top-2'"
      @keydown.escape.exact.prevent.stop="close"
    >
      <span ref="popoverOpenRef" class="hidden" />
      <!-- Name & type -->
      <div class="flex max-w-full flex-row items-center justify-between gap-2">
        <!-- Name -->
        <EditableSpan
          ref="nameRef"
          v-model="name"
          :readonly="readonly"
          class="w-full max-w-full scroll-m-0 overflow-x-hidden rounded-sm border border-orange-900 border-opacity-[12%] p-1 text-gray-900 focus:bg-orange-100"
          @navigate-right="typeButtonRef?.focus()"
          @navigate-down="actionRefs.focus(actions[0].label)"
          @enter="close"
        />
        <!-- Type popover -->
        <div v-if="!isEnum" class="relative">
          <button
            ref="typeButtonRef"
            :disabled="readonly"
            class="rounded-sm border border-orange-900 border-opacity-[12%] p-1 text-gray-900 focus:bg-orange-100 focus:outline-none focus:ring-0"
            :class="readonly ? '' : 'hover:bg-orange-100'"
            @keydown.left.stop.prevent="nameRef?.focus()"
            @keydown.down.stop.prevent="actionRefs.focus(actions[0].label)"
            @click="editingType = true"
            @keydown.enter.stop.prevent="editingType = true"
          >
            <!-- No idea why but this needs to be set absolutely or the icons are too high -->
            <SimpleTypePreview class="absolute top-0.5" :type="value" hide-reference />
          </button>
          <!-- Popover position is also pinned -->
          <div
            v-if="editingType"
            ref="typeEditablePopoverRef"
            class="z-10 flex w-64 flex-col gap-2 rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
            :class="typePopoverPin.pinned.value ? '' : 'absolute -left-1 -top-10'"
          >
            <SelectTypeCell
              :model-value="value"
              @update:model-value="emit('update:modelValue', $event)"
              @escape="
                editingType = false;
                typeButtonRef?.focus();
              "
            />
          </div>
        </div>
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
    </div>
  </div>
</template>
