<script lang="ts" setup>
import { ViewData, NodeType, ViewType, ObjectType, BlockType, FieldData, BoxData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import {
  ViewContentWrapper,
  makeViewId,
  viewEmits,
  type ViewComponent,
  type ViewExposed,
  type ViewProps,
} from "@/views/common";
import { canvas } from "@/system/space";
import { computed, ref, toRef, type Ref } from "vue";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import type { PopoverInfoIn } from "@/utils/menu";
import { ICON_BY_BLOCK_TYPE, IconInline } from "@/system/icon";
import { getStorageKey, getViewForValueType, resolveType, type TypeIdentity } from "@/system/value";
import { getViewComponent } from "@/views/registry";
import { FULL_WIDTH_VIEW_TYPES } from "@/system/lang";

const MIN_WIDTH = 320;
const DEFAULT_WIDTH = 280;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    modelValue?: any;
    size?: Partial<Pick<BoxData, "width" | "height">>;
    preparedConnection?: PreparedGetConnection;
  } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "valueType" | "variant" | "isInput" | "isInline" | "isDisabled">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const componentRefs: Ref<Record<string, ViewComponent | null>> = ref({});
const width = computed(() => Math.max(MIN_WIDTH, props.size?.width ?? DEFAULT_WIDTH));

const hasValue = computed(() => props.modelValue != null);
const baseTypePtr = computed(() => props.valueType?.baseTypePtr as TypedNodeReferenceData<NodeType.BLOCK> | undefined);
const { graph: pkgGraph } = props.preparedConnection ?? useExistingConnection(baseTypePtr);
const baseType = pkgGraph.getRef(baseTypePtr);
const fields = pkgGraph.getChildrenRef(baseType, NodeType.FIELD); // these need to be resolved later :TypeResolution

type FieldView = {
  field: FieldData;
  fieldType: TypeIdentity;
  storageKey: string;
  isSet: boolean;
  value: any;
  component: any | undefined;
  viewType?: ViewType;
  viewProps?: any;
  isFullWidth?: boolean;
};
const fieldViews: Ref<FieldView[]> = computed(() => {
  const fieldViews: FieldView[] = [];
  for (const field of fields.value) {
    const fieldType = resolveType(field, pkgGraph);
    const storageKey = getStorageKey(field, fieldType);
    const fieldValue = props.modelValue?.[storageKey];
    const isSet = fieldValue != null && !(Array.isArray(fieldValue) && fieldValue.length === 0);
    const view = getViewForValueType(fieldType);
    const component = view != null ? getViewComponent(view.viewType) : undefined;
    fieldViews.push({
      field,
      fieldType,
      storageKey,
      isSet,
      value: fieldValue,
      component,
      viewType: view?.viewType,
      viewProps: view?.props,
      isFullWidth: FULL_WIDTH_VIEW_TYPES.includes(view?.viewType!),
    });
  }
  return fieldViews;
});

function focus() {
  if (!props.isInline) {
    return buttonRef.value;
  }
}

function apply(value: any) {
  emit("update:modelValue", value);
  emit("apply", value);
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, focus });
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <!-- Preview -->
    <div
      v-if="!isInline"
      ref="buttonRef"
      v-menu="
        (): PopoverInfoIn => ({
          component: ViewType.OBJECT,
          placement: 'inside-top-left',
          referenceMargin: 0,
          props: {
            ...(props as ViewProps),
            size: { metatype: ObjectType.BOX, width: buttonRef?.getBoundingClientRect().width },
            isInline: true,
          },
          propsRef: () => ({ modelValue: props.modelValue }),
          onUpdate: (value: any) => emit('update:modelValue', value),
          onApply: (value: any) => apply(value),
        })
      "
      role="button"
      :disabled="isDisabled"
      class="group flex w-full flex-row items-center rounded border border-gray-200 px-2.5 py-1 hover:border-gray-300 disabled:bg-gray-100 data-[menu=true]:border-gray-300"
    >
      <!-- Icon/Type name -->
      <IconInline
        v-bind="baseType?.icon ?? ICON_BY_BLOCK_TYPE[BlockType.CLASS]"
        class="mr-1.5 w-5 text-center text-gray-700"
      />
      <span :class="hasValue ? 'text-gray-900' : 'text-gray-400'">{{ baseType?.name ?? "???" }}</span>
      <!-- TODO :UX: Object inline value preview -->
      <div class="ml-2 flex flex-row gap-x-1.5">
        <div v-for="{ field } of fieldViews.filter((f) => f.isSet)" :key="field.id">
          <span class="text-gray-400">{{ field.name }}</span>
        </div>
      </div>
      <!-- Controls -->
      <div class="ml-auto flex-shrink-0 pl-2">
        <!-- Clear -->
        <i
          v-if="hasValue"
          role="button"
          class="fas fa-xmark-circle text-gray-400 opacity-0 hover:text-primary-900 group-hover:opacity-100"
          @click.stop="emit('update:modelValue', null)"
        />
      </div>
    </div>

    <!-- Inline Object -->
    <div v-else>
      <!-- Header? -->
      <div class="flex w-full flex-row border-b px-2.5 py-1">
        <!-- Icon/Type name -->
        <span>
          <IconInline
            v-bind="baseType?.icon ?? ICON_BY_BLOCK_TYPE[BlockType.CLASS]"
            class="mr-1.5 w-5 text-center text-gray-700"
          />
          <span class="font-semibold">{{ baseType?.name }}</span>
        </span>
        <!-- Controls -->
        <div class="ml-auto flex-shrink-0 pl-2 pr-2">
          <!-- Clear -->
          <i
            v-if="hasValue"
            role="button"
            class="fas fa-xmark-circle text-gray-400 hover:text-primary-900"
            @click.stop="emit('update:modelValue', null)"
          />
        </div>
      </div>
      <!-- Fields -->
      <ul class="flex w-full flex-col gap-y-2.5 py-3" :style="{ width: width + 'px' }">
        <li
          v-for="{ field, storageKey, value, viewType, component, isFullWidth, viewProps } of fieldViews"
          :key="field.id"
          class="mx-auto w-full px-4"
          :class="[isFullWidth ? 'flex flex-col' : 'flex flex-row flex-wrap items-center gap-x-[10%]']"
          :style="{ minWidth: MIN_WIDTH + 'px' }"
        >
          <!-- Field -->
          <span class="w-[100px]">
            <span class="max-w-full truncate py-1 font-medium text-gray-700">{{ field.name }}</span>
          </span>
          <!-- Value -->
          <component
            :is="component"
            v-if="component"
            :ref="(ref: any) => (ref != null ? (componentRefs[field.id] = ref) : delete componentRefs[field.id])"
            :class="['ml-auto flex-shrink-0', isFullWidth ? '' : 'text-right']"
            :style="{ width: isFullWidth ? '100%' : 'calc(90% - 100px)' }"
            v-bind="viewProps"
            :model-value="value"
            @update:model-value="
              (value: any) => {
                const newValue = { ...props.modelValue, [storageKey]: value };
                emit('update:modelValue', newValue);
              }
            "
          />
          <div v-else class="ml-auto text-warning-600">
            {{ viewType != null ? ViewType[viewType] : "No View for Value" }}
          </div>
        </li>
      </ul>
    </div>
  </ViewContentWrapper>
</template>
