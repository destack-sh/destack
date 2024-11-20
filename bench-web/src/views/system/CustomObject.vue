<script lang="ts" setup>
import {
  BlockType,
  RectangleData,
  FieldType,
  IconData,
  NodeType,
  ObjectType,
  TypeKind,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { ICON_BY_BLOCK_TYPE, IconInline } from "@/ui/icon";
import { canvas } from "@/system/space";
import { getFieldViews } from "@/ui/view";
import type { PopoverInfoIn } from "@/ui/popover";
import { ViewContentWrapper, viewEmits, type ViewComponent, type ViewExposed, type ViewProps } from "@/views/common";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { computed, ref, toRef, type Ref } from "vue";
import { getTitleField } from "@/language/field";
import { packValue, unpackValue } from "@/language/value";

const MIN_WIDTH = 320;
const DEFAULT_WIDTH = 280;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    modelValue?: any;
    size?: Partial<Pick<RectangleData, "width" | "height">>;
    preparedConnection?: PreparedGetConnection;
  } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "valueType" | "variant" | "isInput" | "isInline" | "isDisabled">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");

const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const componentRefs: Ref<Record<string, ViewComponent | null>> = ref({});
const width = computed(() =>
  props.variant == Variant.STEALTH ? null : Math.max(MIN_WIDTH, props.size?.width ?? DEFAULT_WIDTH),
);

const facetIcon = computed(() => baseType.value?.icon ?? ICON_BY_BLOCK_TYPE[BlockType.CLASS]);
const facetName = computed(() => baseType.value?.name);
const baseTypePtr = computed(() => props.valueType?.baseTypePtr as TypedNodeReferenceData<NodeType.BLOCK> | undefined);
const { graph: pkgGraph } = props.preparedConnection ?? useExistingConnection(baseTypePtr);
const baseType = pkgGraph.getRef(baseTypePtr);
const fields = pkgGraph.getChildrenRef(baseType, NodeType.FIELD); // these need to be resolved later :TypeResolution
const titleField = computed(() => getTitleField(fields.value));
const fieldViews = computed(() =>
  getFieldViews(fields.value, pkgGraph, {
    types: [props.valueType?.baseFieldType ?? FieldType.MEMBER],
    isInput: props.isInput,
  }),
);
const hasValue = computed(() => {
  if (props.modelValue == null) return false;
  if (props.valueType?.isList) return (props.modelValue as any[]).length > 0;
  else return true;
});
const values: Ref<{ title: string | number | undefined; icon: IconData | undefined; value: any }[]> = computed(() => {
  if (!hasValue.value) return [];
  const values = props.valueType?.isList ? (props.modelValue as any[]) : [props.modelValue];
  return values.map((value) => {
    const title = titleField.value != null ? value[fieldViews.value[titleField.value.idx!]?.storageKey] : undefined;
    return { title, icon: undefined, value };
  });
});
const activeValueIdx: Ref<number | null> = ref(null);
const focusedValueIdx = computed(() => {
  if (!props.valueType?.isList || activeValueIdx.value == null) return 0;
  else return activeValueIdx.value;
});
const focusedValue = computed(() => values.value[focusedValueIdx.value!]?.value);

function focus() {
  if (!props.isInline) {
    return buttonRef.value;
  }
}

function add() {
  if (!props.valueType?.isList) throw new Error(`cannot add to non-list`);
  apply(((props.modelValue as any[]) ?? []).concat({}));
}
function remove(idx: number) {
  if (!props.valueType?.isList) throw new Error(`cannot remove from non-list`);
  apply(((props.modelValue as any[]) ?? []).filter((_, i) => i != idx));
}
function apply(value: any) {
  emit("update:modelValue", value);
  emit("apply", value);
}
function clear() {
  apply(props.valueType?.isList ? [] : undefined);
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, focus });
</script>
<template>
  <ViewContentWrapper :type="ViewType.OBJECT" v-bind="props">
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
            size: {
              metatype: ObjectType.RECTANGLE,
              width: Math.max(MIN_WIDTH, buttonRef?.getBoundingClientRect().width!),
            },
            isInline: true,
          },
          propsRef: () => ({ modelValue: props.modelValue }),
          onUpdate: (value: any) => emit('update:modelValue', value),
          onApply: (value: any) => apply(value),
        })
      "
      role="button"
      :disabled="isDisabled"
      class="group flex w-full flex-row flex-wrap items-center gap-y-1 rounded border border-gray-200 bg-white px-2.5 py-1 hover:border-gray-200 disabled:bg-gray-100 data-[popover=true]:border-gray-200"
    >
      <!-- Current value -->
      <template v-if="hasValue">
        <button
          v-for="(v, i) in values"
          :key="i"
          class="mr-2 flex flex-row items-center rounded"
          :class="valueType?.isList ? 'bg-gray-100 px-1' : ''"
        >
          <IconInline v-bind="facetIcon" class="mr-1.5 w-5 text-center text-gray-700" />
          <span class="max-w-28 truncate text-gray-900">{{ v.title ?? facetName ?? "???" }}</span>
        </button>
      </template>
      <div v-else class="mr-2">
        <IconInline v-bind="facetIcon" class="mr-1.5 w-5 text-center text-gray-400" />
        <span class="text-gray-400">{{ facetName ?? "???" }}</span>
      </div>
      <!-- Add -->
      <button
        v-if="!isDisabled && isInput && valueType?.isList"
        class="mr-2 text-gray-400 opacity-0 hover:text-gray-700 group-hover:opacity-100"
        @click.stop="add"
      >
        <i class="fas fa-plus" />
      </button>
      <!-- Controls -->
      <div v-if="!isDisabled && isInput" class="ml-auto flex-shrink-0 pl-2">
        <!-- Clear -->
        <button
          v-if="hasValue && !valueType?.isRequired"
          class="text-gray-400 opacity-0 hover:text-gray-700 group-hover:opacity-100"
          @click.stop="clear"
        >
          <i class="fas fa-xmark" />
        </button>
      </div>
    </div>

    <!-- Inline Object -->
    <div v-else class="w-full">
      <!-- Header -->
      <div
        v-if="variant != Variant.STEALTH"
        class="flex w-full flex-row flex-wrap items-center gap-y-1 border-b px-2.5 py-1"
      >
        <!-- Current value -->
        <template v-if="hasValue">
          <button
            v-for="(v, i) in values"
            :key="i"
            class="mr-2 flex flex-row items-center rounded bg-gray-100 px-1"
            @click.stop="activeValueIdx = i"
          >
            <IconInline
              v-bind="facetIcon"
              class="mr-1.5 w-5 text-center"
              :class="activeValueIdx == i ? 'text-primary-700' : 'text-gray-700'"
            />
            <span
              class="max-w-28 truncate"
              :class="activeValueIdx == i ? 'font-medium text-gray-900' : 'text-gray-900'"
            >
              {{ v.title ?? facetName ?? "???" }}
            </span>
            <!-- Remove -->
            <button
              v-if="!isDisabled && isInput && valueType?.isList"
              class="ml-1.5 text-gray-400 hover:text-gray-700"
              @click.stop="remove(i)"
            >
              <i class="fas fa-xmark" />
            </button>
          </button>
        </template>
        <template v-else>
          <IconInline v-bind="facetIcon" class="mr-1.5 w-5 text-center text-gray-400" />
          <span class="max-w-28 truncate text-gray-400">{{ facetName ?? "???" }}</span>
        </template>
        <!-- Add -->
        <button
          v-if="!isDisabled && isInput && valueType?.isList"
          class="mr-2 text-gray-400 hover:text-gray-700"
          @click.stop="add"
        >
          <i class="fas fa-plus" />
        </button>
        <!-- Controls -->
        <div v-if="!isDisabled && isInput" class="ml-auto flex-shrink-0 pl-2 pr-2">
          <!-- Clear -->
          <button class="mr-2 text-gray-400 hover:text-gray-700" @click.stop="clear">
            <i class="fas fa-xmark" />
          </button>
        </div>
      </div>
      <!-- Fields (for current value) -->
      <ul
        class="flex flex-col gap-y-2.5"
        :class="variant != Variant.STEALTH ? 'py-3' : ''"
        :style="{ width: width != null ? width + 'px' : '100%' }"
      >
        <li
          v-for="fieldView of fieldViews"
          :key="fieldView.field.id"
          class="mx-auto w-full"
          :class="[
            fieldView.isFullWidth ? 'flex flex-col' : 'flex flex-row items-center gap-x-[10%]',
            variant != Variant.STEALTH ? 'px-4' : '',
          ]"
          :style="{ minWidth: MIN_WIDTH + 'px' }"
        >
          <!-- Field -->
          <span class="w-[100px]">
            <span class="max-w-full truncate py-1 text-gray-900">{{ fieldView.field.name }}</span>
          </span>
          <!-- Value -->
          <component
            :is="getViewComponent(fieldView.viewType)"
            v-if="
              (focusedValue?.[fieldView.storageKey] != null || (isInput && !isDisabled)) &&
              fieldView.viewType != null &&
              hasViewComponent(fieldView.viewType)
            "
            :id="fieldView.field.id + '.value'"
            :ref="
              (ref: any) =>
                ref != null ? (componentRefs[fieldView.field.id] = ref) : delete componentRefs[fieldView.field.id]
            "
            :class="['ml-auto flex-shrink-0', fieldView.isFullWidth ? '' : 'text-right']"
            :style="{ width: fieldView.isFullWidth ? '100%' : 'calc(90% - 100px)' }"
            v-bind="fieldView.viewProps"
            :model-value="
              fieldView.field.kind == TypeKind.OBJECT
                ? focusedValue?.[fieldView.storageKey]
                : unpackValue(focusedValue?.[fieldView.storageKey], fieldView.field, {
                    graph: pkgGraph,
                    wrapScalar: false,
                    recurseCustomObject: false,
                  })
            "
            @update:model-value="
              (value: any) => {
                const valuePacked =
                  fieldView.field.kind == TypeKind.OBJECT
                    ? value
                    : packValue(value, fieldView.field, {
                        graph: pkgGraph,
                        wrapScalar: false,
                        recurseCustomObject: false,
                      });
                const newObject = { ...focusedValue, [fieldView.storageKey]: valuePacked };
                if (!valueType?.isList) {
                  emit('update:modelValue', newObject);
                } else {
                  const newValues = (props.modelValue as any[]).map((v, i) => (i == focusedValueIdx ? newObject : v));
                  emit('update:modelValue', newValues);
                }
              }
            "
          />
          <div
            v-else-if="fieldView.viewType != null"
            class="ml-auto flex-shrink-0 text-gray-400"
            :class="fieldView.isFullWidth ? '' : 'text-right'"
          >
            <span class="italic">Unset</span>
          </div>
          <div v-else class="ml-auto flex-shrink-0 text-warning-600" :class="fieldView.isFullWidth ? '' : 'text-right'">
            {{ fieldView.viewType != null ? ViewType[fieldView.viewType] : "No View for Type" }}
          </div>
        </li>
      </ul>
    </div>
  </ViewContentWrapper>
</template>
