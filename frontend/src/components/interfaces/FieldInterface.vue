<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import SelectTypeInterface from "@/components/interfaces/SelectTypeInterface.vue";
import TypePreview from "@/components/interfaces/TypePreview.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { ANY_FIELD, getEnumColor } from "@/state/statement";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { useElementSize } from "@/composables/useSize";
import type { TypeAction } from "@/state/bench";
import { setDragData, useRelativeDropZone, type Dragged } from "@/utils/drop";
import { syncProperty } from "@/utils/sync";
import {
  AdjustmentsHorizontalIcon,
  ArrowDownIcon,
  ArrowUpIcon,
  FunnelIcon,
  Square2StackIcon,
} from "@heroicons/vue/24/outline";
import TrashIcon from "@heroicons/vue/24/outline/TrashIcon";
import { computed, nextTick, ref, watch, type Ref } from "vue";
import { SortOp, TypeTag } from "@/gql/graphql";
import { canSort } from "@/state/type";
import { useCurrentModule, type Field, TypeFlag } from "@/state/module";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";

const props = defineProps<{
  modelValue?: Field;
  readonly: boolean;
  inlined?: boolean;
  refOnly?: boolean;
  refTypes?: TypeTag[];
  hideFlags?: boolean;
  extraActions?: TypeAction[];
  tupleName?: string;
  isEnum?: boolean;
  isView?: boolean;
  hideOutline?: boolean;
  hideText?: boolean;
  canFilter?: boolean;
  orientation?: "horizontal" | "vertical";
  statementId?: string;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Pick<Field, "name" | "tag" | "flags" | "referenceCk">): void;
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
  (e: "sort", order: SortOp): void;
  (e: "filter"): void;
}>();

const tupleName = computed(() => props.tupleName ?? "field");
const value: Ref<Field> = ref(props.modelValue ?? ANY_FIELD);
const name: Ref<string> = ref(props.modelValue?.name ?? "");
const text: Ref<string> = ref(props.modelValue?.text ?? "");
const editing = ref(false);
const editingType = ref(false);
const hasText = computed(() => text.value.length > 0);
const module = useCurrentModule();
const runtimeType = computed(() =>
  props.modelValue?.referenceCk == null ? null : module.statementOf(props.modelValue?.referenceCk)
);

const containerRef: Ref<HTMLDivElement | null> = ref(null);
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const previewRef: Ref<HTMLDivElement | null> = ref(null);
const nameRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const textRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const typeButtonRef: Ref<HTMLButtonElement | null> = ref(null);
const actionRefs = useElementRefs();
const previewSize = useElementSize(previewRef);

// pin popover to the right
const editablePopoverRef: Ref<HTMLDivElement | null> = ref(null);
const popoverPin = pinAbsoluteElement(editablePopoverRef, { pos: true, keepInView: true });
const typeEditablePopoverRef = ref<HTMLDivElement | null>(null);
const typePopoverPin = pinAbsoluteElement(typeEditablePopoverRef, { pos: true, keepInView: true });

// sync name/text
const nameSync = syncProperty({
  read: () => (text.value = value.value.text ?? ""),
  write: () => {
    value.value = {
      ...value.value,
      name: name.value,
      text: text.value,
    };
    emit("update:modelValue", value.value);
  },
});

// sync model value into local value
watch(
  () => [props.modelValue],
  () => {
    value.value = props.modelValue ?? ANY_FIELD;
  }
);

// actions

const actions: Ref<TypeAction[]> = computed(() => {
  const actions = [];
  if (!props.readonly) {
    if (!props.isEnum) {
      actions.push({
        groupId: "general",
        label: "Edit type",
        icon: AdjustmentsHorizontalIcon,
        keepOpen: true,
        action: () => typeButtonRef.value?.click(),
      });
    }
    actions.push({
      groupId: "general",
      label: "Duplicate " + tupleName.value,
      icon: Square2StackIcon,
      action: () => emit("duplicateSelf"),
    });
    actions.push({
      groupId: "general",
      label: "Delete " + tupleName.value,
      icon: TrashIcon,
      action: () => emit("deleteSelf"),
    });
    if (props.isView) {
      actions.push({
        groupId: "query",
        label: "Sort ascending",
        icon: ArrowUpIcon,
        action: () => emit("sort", SortOp.Ascending),
        disabled: !canSort(value.value),
      });
      actions.push({
        groupId: "query",
        label: "Sort descending",
        icon: ArrowDownIcon,
        action: () => emit("sort", SortOp.Descending),
        disabled: !canSort(value.value),
      });
      actions.push({
        groupId: "query",
        label: "Filter",
        icon: FunnelIcon,
        disabled: !props.canFilter,
        action: () => {
          emit("filter");
        },
      });
    }
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

function onDrop(thing: File[] | Dragged | null) {
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

function open(select?: "all") {
  if (props.readonly) return;
  if (!editing.value) {
    editing.value = true;
    if (!props.readonly) {
      nextTick(() => {
        nameRef.value?.focus();
        if (select == "all") {
          nameRef.value?.selectAll();
        }
      });
    }
  }
}

function focus() {
  buttonRef.value?.focus();
}

function blur() {
  buttonRef.value?.blur();
}

function close(refocus = true) {
  editing.value = false;
  editingType.value = false;
  if (refocus) {
    nextTick(focus);
  }
}

defineExpose({
  editing,
  focus,
  blur,
  open,
  close,
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
      class="group/preview mousetrap-no-tab relative flex h-full w-full flex-col text-left outline-none"
      @keydown.left.exact.prevent="emit('navigateLeft')"
      @keydown.right.exact.prevent="emit('navigateRight')"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
      @keydown.delete.exact="editing || emit('deleteSelf')"
      @keydown.enter.exact.prevent="open()"
      @keydown.tab.exact.prevent="editing || emit('navigateRight')"
      @keydown.shift.tab.exact.prevent="editing || emit('navigateLeft')"
      @click.stop="open()"
      @contextmenu.prevent.stop="open()"
      @dragstart.stop="onDragStart"
      @mousedown="buttonRef?.setAttribute('draggable', 'true')"
      @mouseup="buttonRef?.setAttribute('draggable', 'false')"
    >
      <!-- Inner div so we can keep the button at the right height without the items-center below centering everything vertically -->
      <!-- (and measure the inner preview ref size correctly) -->
      <div class="flex max-w-full flex-row items-baseline">
        <!-- Type signature (type + name) -->
        <div
          ref="previewRef"
          class="flex max-w-full flex-row items-center rounded-sm text-left"
          :class="[isEnum || !hideOutline ? 'py-0.5 pr-2' : '']"
        >
          <!-- :EnumStyle -->
          <svg
            v-if="isEnum"
            class="absolute left-1 top-[8px] h-[8px] w-[8px]"
            :style="{ fill: getEnumColor(value) }"
            viewBox="0 0 6 6"
            aria-hidden="true"
          >
            <rect rx="2" ry="2" width="5" height="6" />
          </svg>
          <TypePreview v-else :type="value" class="absolute left-0" />
          <span
            class="ml-5 max-w-full truncate font-semibold text-gray-800"
            :class="[inlined ? 'underline decoration-fuchsia-300 underline-offset-4' : '', isEnum ? 'ml-4 ' : '']"
            >{{ value.name }}</span
          >
          <!-- Type flags (obviously not pretty, like everything else...) -->
          <span
            v-if="value.tag != TypeTag.Literal && value.tag != TypeTag.Boolean && !(value.flags & TypeFlag.IS_OPTIONAL)"
            class="ml-0.5 text-gray-700"
            >!</span
          >
          <span v-if="value.flags & TypeFlag.IS_ARRAY" class="ml-0.5 text-gray-700">[]</span>
          <!-- Type reference name -->
          <span v-if="value.referenceCk" class="ml-1.5 text-gray-400">{{ runtimeType?.name }}</span>
        </div>
        <!-- Text -->
        <AnnotatedText
          v-if="text && !hideText"
          :model-value="text"
          readonly
          minimal-mentions
          class="ml-2 flex-shrink flex-grow-0 truncate text-gray-400"
        />
      </div>
    </button>
    <!-- Prevent scroll and capture click outside -->
    <div v-if="editing" class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="close()" />
    <!-- Edit popover -->
    <!-- Popover position is pinned -->
    <FadeTransition>
      <div
        v-if="editing"
        ref="editablePopoverRef"
        class="z-50 flex w-72 flex-col rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :class="[popoverPin.pinned.value ? '' : 'absolute -top-2', isEnum ? '-left-0' : '-left-2']"
        @keydown.escape.exact.prevent.stop="close()"
      >
        <!-- Name & type -->
        <div class="relative flex max-w-full flex-row items-center justify-between gap-2">
          <!-- Type popover -->
          <FadeTransition>
            <div v-if="!isEnum" class="relative">
              <button
                ref="typeButtonRef"
                :disabled="readonly"
                class="rounded-sm border border-orange-900/[12%] p-1 text-gray-900 focus:bg-orange-100 focus:outline-none focus:ring-0"
                :class="readonly ? '' : 'hover:bg-orange-100'"
                @keydown.left.stop.prevent="nameRef?.focus()"
                @keydown.down.stop.prevent="actionRefs.focus(actions[0].label)"
                @click="editingType = true"
                @contextmenu.prevent.stop="editingType = true"
                @keydown.enter.stop.prevent="editingType = true"
              >
                <!-- No idea why but this needs to be set absolutely or the icons are too high -->
                <TypePreview class="top-0.5" :type="value" />
              </button>
              <!-- Popover position is also pinned -->
              <div
                v-if="editingType"
                ref="typeEditablePopoverRef"
                class="z-10 flex w-80 flex-col gap-2 rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
                :class="typePopoverPin.pinned.value ? '' : 'absolute -right-1 -top-10'"
              >
                <div class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="editingType = false" />
                <SelectTypeInterface
                  :model-value="value"
                  @update:model-value="emit('update:modelValue', $event)"
                  :ref-only="refOnly"
                  :ref-types="refTypes"
                  class="z-50 w-full"
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
              class="rounded-sm border border-orange-900/[12%] p-2 hover:bg-orange-100 focus:bg-orange-100 focus:outline-none focus:ring-0"
            >
              <svg class="h-3 w-3" :style="{ fill: getEnumColor(value) }" viewBox="0 0 6 6" aria-hidden="true">
                <rect x="0" y="0" width="6" height="6" />
              </svg>
            </div>
          </FadeTransition>
          <!-- Name -->
          <EditableSpan
            ref="nameRef"
            regex="name"
            v-model="name"
            @update:model-value="nameSync.onLocalWrite"
            :readonly="readonly"
            class="w-full max-w-full scroll-m-0 overflow-x-hidden rounded-sm border border-orange-900/[12%] p-1 text-gray-900 focus:bg-orange-100"
            @navigate-right="typeButtonRef?.focus()"
            @navigate-down="textRef?.focus()"
            @enter="close(false), emit('enter')"
            @enter-left="close(false), emit('enter')"
            @enter-right="close(false), emit('enter')"
          />
          <!-- Name placeholder -->
          <span v-if="!value.name" class="absolute left-[5px] top-[5px] text-gray-400" @click="nameRef?.focus"
            >{{ tupleName }} name</span
          >
        </div>
        <!-- Text -->
        <span
          class="max-w-fullrounded-sm relative mt-1 w-full p-1 text-gray-700 focus-within:bg-orange-100 hover:bg-orange-100"
        >
          <AnnotatedText
            ref="textRef"
            regex="description"
            v-model="text"
            @update:model-value="nameSync.onLocalWrite"
            :readonly="readonly"
            class="w-full max-w-full scroll-m-0 overflow-x-hidden whitespace-normal"
            @navigate-up="nameRef?.focus()"
            @navigate-right="typeButtonRef?.focus()"
            @navigate-down="actionRefs.focus(actions[0].label)"
            @enter-left="close(false), emit('enter')"
            @enter="close(false), emit('enter')"
            @enter-right="close(false), emit('enter')"
          />
          <!-- Text placeholder -->
          <span v-if="!hasText" class="text-gray-400" @click="textRef?.focus">Describe {{ tupleName }} with text</span>
        </span>
        <!-- Actions -->
        <div class="mt-0.5 flex flex-col" v-if="actions.length > 0">
          <button
            v-for="(action, i) in actions"
            :ref="(el: any) => actionRefs.registerRef(action.label, el)"
            :key="action.label"
            class="flex w-full flex-row items-center gap-2.5 rounded-sm px-1 py-1 hover:bg-orange-100 focus:bg-orange-100 focus:outline-none"
            :class="[
              i > 0 && actions[i - 1].groupId != action.groupId ? 'mt-1 border-t border-orange-900/[12%] pt-2' : '',
              action.disabled ? 'cursor-not-allowed opacity-50' : '',
            ]"
            :disabled="action.disabled"
            @click="
              action.action(value);
              action.keepOpen || close();
            "
            @keydown.enter.prevent.stop="
              action.action(value);
              action.keepOpen || close();
            "
            @keydown.up.exact.stop.prevent="i == 0 ? textRef?.focus() : actionRefs.focus(actions[i - 1].label)"
            @keydown.down.exact.stop.prevent="i == actions.length - 1 ? null : actionRefs.focus(actions[i + 1].label)"
          >
            <component :is="action.icon" class="h-4 w-4 text-gray-700" />
            <span class="text-gray-700">{{ action.label }}</span>
          </button>
        </div>
      </div>
    </FadeTransition>
  </div>
</template>
