<script lang="ts" setup>
import { NodeType, Variant, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionMapImplementation } from "@/ui/action";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { IconInline, getNodeIcon } from "@/ui/icon";
import { NAME_CONSTRAINT } from "@/language/utils";
import { canvas, inspectionPtr } from "@/system/space";
import { getNativeConstraintProps, guardNativeInput } from "@/ui/view";
import { type PopoverInfoIn } from "@/ui/popover";
import type { TooltipInfo } from "@/ui/tooltip";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import Icon from "@/views/content/Icon.vue";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Pick<
    ViewData,
    "variant" | "nodePtr"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const fieldRef = ref<HTMLElement | null>(null);
const nameRef = ref<HTMLElement | null>(null);

const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.FIELD>);
const { graph: pkgGraph, connection: pkgConnection } = props.preparedConnection ?? useExistingConnection(nodePtr);
const field = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });

// actions
const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"type"> = {
  // type
  "type.edit.isList": {
    isChecked: () => field.value?.isList ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isList: !field.value!.isList });
    },
  },
  "type.edit.isRequired": {
    isChecked: () => field.value?.isRequired ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isRequired: !field.value!.isRequired });
    },
  },
  "type.edit.isSecret": {
    isChecked: () => field.value?.isSecret ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isSecret: !field.value!.isSecret });
    },
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    v-if="field"
    ref="fieldRef"
    class="flex w-fit flex-row items-center rounded border bg-gray-100 px-1.5 py-[3px]"
    :class="[
      inspectionPtr?.id == field.id
        ? 'border-primary-900'
        : [variant != Variant.STEALTH ? 'border-gray-200' : 'border-gray-200', 'hover:border-gray-300'],
    ]"
  >
    <!-- TODO :UX!: Field is annoying (should be double-click to edit, change type in contextmenu, etc.) -->
    <IconInline
      v-tooltip="{ small: true, text: `Change icon` } as TooltipInfo"
      v-menu="
        (): PopoverInfoIn => ({
          component: Icon,
          placement: 'bottom-right',
          offset: '-referenceWidth',
          props: { modelValue: field!.icon },
          onApply: (newIcon) => pkgConnection.tx.update(field!, { icon: newIcon }),
        })
      "
      v-bind="getNodeIcon(field)"
      class="mr-1.5 w-5 rounded p-0.5 text-gray-700 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
    />
    <input
      ref="nameRef"
      type="text"
      class="w-fit min-w-fit max-w-fit truncate rounded border-0 bg-transparent font-medium outline-none ring-0 hover:bg-gray-100 focus:ring-0"
      spellcheck="false"
      :value="field.name"
      :size="(field.name?.length ?? 0) + 3"
      v-bind="getNativeConstraintProps(NAME_CONSTRAINT)"
      @input="
        guardNativeInput(NAME_CONSTRAINT, $event, field.name, (newValue) =>
          pkgConnection.tx.update(field!, { name: newValue }, { debounce: 'long' }),
        )
      "
    />
  </div>
  <Inaccessible v-else class="bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
