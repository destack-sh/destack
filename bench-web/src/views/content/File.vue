<script lang="ts" setup>
import { ViewData, NodeType, FileReferenceData, ViewType, FileType } from "@/proto/wire";
import { describeNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, ViewContentWrapper, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, toRef } from "vue";
import { toCamelName } from "@/system/lang";
import { ICON_BY_FILE_TYPE, IconInline } from "@/system/icon";
import { FILE_TYPE_BY_VIEW_TYPE } from "@/system/file";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: FileReferenceData } & Partial<
    Pick<
      ViewData,
      | "type"
      | "name"
      | "title"
      | "text"
      | "icon"
      | "valueType"
      | "nodePtr"
      | "variant"
      | "isInput"
      | "isInline"
      | "isDisabled"
    >
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const fileType = computed(() => {
  if (props.type != null) return FILE_TYPE_BY_VIEW_TYPE[props.type] ?? FileType.GENERIC;
  else if (props.valueType?.constraint?.fileType != null) return props.valueType.constraint.fileType;
  else return FileType.GENERIC;
});

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <!-- nocheckin: File -->
    <!-- Dropdown -->
    <button
      v-if="!isInline"
      ref="buttonRef"
      class="group flex w-full flex-row items-center rounded border border-gray-200 px-2.5 py-1 hover:border-gray-300 data-[popover=true]:border-gray-300"
    >
      <!-- Current value -->
      <template v-if="modelValue != null">
        <IconInline v-bind="ICON_BY_FILE_TYPE[fileType]" class="mr-1.5 w-5 text-gray-700" />
        {{ modelValue.title ?? '???' }}
      </template>
      <span v-else class="select-none text-gray-500">
        <IconInline v-bind="ICON_BY_FILE_TYPE[fileType]" class="mr-1.5 w-5 text-gray-700" />
        Upload {{ toCamelName(ViewType, type ?? ViewType.FILE) }}
      </span>
      <!-- Controls -->
      <div v-if="!props.isDisabled && props.isInput" class="ml-auto flex-shrink-0 pl-1.5">
        <!-- Clear -->
        <i
          v-if="modelValue != null && !valueType?.isRequired"
          role="button"
          class="fas fa-xmark mr-2 text-gray-400 opacity-0 hover:text-primary-900 group-hover:opacity-100"
          @click.stop="emit('update:modelValue', undefined)"
        />
        <i class="fas fa-caret-down ml-auto text-gray-400 hover:text-primary-900" />
      </div>
    </button>
  </ViewContentWrapper>
</template>
