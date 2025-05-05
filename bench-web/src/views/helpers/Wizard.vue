<script lang="ts" setup>
import { makeType } from "@/language/core/type";
import {
  BenchType,
  ButtonVariant,
  Continent,
  NodeType,
  PrimitiveType,
  TypeKind,
  UserWizardViewStage,
  ViewData,
  type NodeReferenceData,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { benchPtr } from "@/system/client";
import { canvas, goToBench } from "@/system/space";
import { logIn, signUp, user } from "@/system/user";
import { fireCommandById } from "@/ui/command";
import { makeIcon } from "@/ui/icon";
import { DEFAULT_REGION_BY_CONTINENT } from "@/utils/region";
import { getNow, TimeUpdateInterval } from "@/utils/time";
import { type FocusAnchor, type ViewEmits, type ViewExpose } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
import Picker from "@/views/content/Picker.vue";
import Button from "@/views/controls/Button.vue";
import ThreeIcon from "@/views/helpers/ThreeIcon.vue";
import { whenever } from "@vueuse/core";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const props = defineProps<{ self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<ViewData, "title">>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");

// state
const stage = ref<UserWizardViewStage>(UserWizardViewStage.LOG_IN);
const name: Ref<string> = ref("");
const slug: Ref<string> = ref("");
const email: Ref<string> = ref("");
const password: Ref<string> = ref("");
const continent: Ref<Continent> = ref(Continent.EUROPE);
const inviteCode: Ref<number | null> = ref(null);
// NOTE: auto-set geolocation once we have more than once region :MultiRegion
// watchEffect(() => {
//   if (GEOLOCATION.value?.continent != null) {
//     const defaultRegion = DEFAULT_REGION_BY_AREA[GEOLOCATION.value.continent];
//     if (defaultRegion != null) {
//       continent.value = defaultRegion;
//     }
//   }
// });
const isActive = ref(false);
const lastError: Ref<string | null> = ref(null);
const isCreating = ref(false);

// invite codes
const now = getNow(TimeUpdateInterval.SECOND);
const isInviteCodeValid = computed(() => {
  // invite code must be <time>*<factor> -> last 4 digits
  if (inviteCode.value == null) return false;
  const factor = 1733;
  const timeAsStr = now.value.toFormat("HHmm");
  const expectedNum = parseInt(timeAsStr) * factor;
  const expected = expectedNum.toString().slice(-4);
  const isValid = inviteCode.value.toString() === expected;
  return isValid;
});

function clear() {
  name.value = "";
  slug.value = "";
  email.value = "";
  password.value = "";
  inviteCode.value = null;
}

function switchStage() {
  if (stage.value == UserWizardViewStage.LOG_IN) {
    stage.value = UserWizardViewStage.SIGN_UP;
  } else if (stage.value == UserWizardViewStage.SIGN_UP) {
    stage.value = UserWizardViewStage.LOG_IN;
  } else {
    throw new Error(`unexpected registration stage: ${stage.value}`);
  }
  nextTick(() => focus());
}

async function submit() {
  isActive.value = true;
  lastError.value = null;
  try {
    if (stage.value == UserWizardViewStage.SIGN_UP) {
      isCreating.value = true;
      try {
        const region = DEFAULT_REGION_BY_CONTINENT[continent.value];
        if (region == null) {
          throw new Error(`unexpected continent: ${continent.value}`);
        }
        const { user } = await signUp(
          { name: name.value, slug: slug.value, email: email.value, region: region },
          password.value,
        );
        await goToBench({ bench: user.benchPtr as TypedNodeReferenceData<NodeType.BENCH> });
        await fireCommandById("space.create.page");
      } finally {
        isCreating.value = false;
      }
    } else if (stage.value == UserWizardViewStage.LOG_IN) {
      const { user } = await logIn({ slug: slug.value }, password.value);
      // if we're outside a Bench and have a Bench, go home
      if (user.benchPtr != null && benchPtr.value == null) {
        await goToBench({ bench: user.benchPtr as TypedNodeReferenceData<NodeType.BENCH> });
      }
    } else {
      throw new Error(`unexpected registration stage: ${stage.value}`);
    }
    clear();
  } catch (e) {
    lastError.value = (e as Error).message ?? "Unknown error";
  } finally {
    isActive.value = false;
  }
}

// view
const nameRef = ref<InstanceType<typeof NativeInput> | null>(null);
const slugRef = ref<InstanceType<typeof NativeInput> | null>(null);
const emailRef = ref<InstanceType<typeof NativeInput> | null>(null);

// autofocus
whenever(
  () => [nameRef.value, slugRef.value, emailRef.value],
  () => {
    nextTick(() => focus());
  },
);

function focus(anchor?: FocusAnchor | NodeReferenceData) {
  (nameRef.value ?? slugRef.value ?? emailRef.value)?.focus?.();
}

defineExpose<ViewExpose>({ id, self, focus });
</script>
<template>
  <div class="flex h-full flex-col divide-x divide-gray-200 lg:flex-row">
    <!-- TODO :UX!: make UserWizard not suck -->
    <!-- Image -->
    <div
      class="flex h-[30%] w-full shrink-0 flex-col items-center justify-center bg-gradient-to-br from-yellow-400 to-amber-400 lg:h-full lg:w-[40%]"
    >
      <ThreeIcon class="h-[50%] w-[50%] lg:h-[40%] lg:w-[40%]" />
    </div>

    <!-- Body -->
    <div
      class="mx-auto my-12 flex h-full w-full max-w-lg flex-1 flex-col overflow-hidden rounded-sm px-9 text-left text-gray-900 lg:justify-center"
    >
      <!-- Header -->
      <div class="mb-4">
        <h2 class="mb-2 text-3xl font-semibold lg:text-4xl">Bench</h2>
        <p class="text-gray-700">Personal software at the cost of compute.</p>
      </div>
      <!-- Form -->
      <form v-if="!user" @submit.prevent="">
        <div class="mt-5 space-y-2">
          <div v-if="stage == UserWizardViewStage.SIGN_UP">
            <div class="mb-1 block text-gray-700">Name</div>
            <NativeInput
              id="name"
              ref="nameRef"
              v-model="name"
              :icon="makeIcon({ faName: 'fas fa-user' })"
              name="Name"
              title="Name"
              placeholder="Florian Cäsar"
              is-input
            />
          </div>
          <div>
            <div class="mb-1 block text-gray-700">Username</div>
            <NativeInput
              id="slug"
              ref="slugRef"
              v-model="slug"
              :icon="makeIcon({ faName: 'fas fa-hashtag' })"
              name="slug"
              title="Username"
              placeholder="florian"
              is-input
            />
          </div>
          <div v-if="stage == UserWizardViewStage.SIGN_UP">
            <div class="mb-1 block text-gray-700">Email</div>
            <NativeInput
              id="email"
              v-model="email"
              :icon="makeIcon({ faName: 'fas fa-at' })"
              name="Email"
              title="Email"
              placeholder="florian@symbolx.com"
              is-input
            />
          </div>
          <!-- NOTE :UX: add passowrd feedback (see https://zxcvbn-ts.github.io/zxcvbn/)? -->
          <div>
            <div class="mb-1 block text-gray-700">Password</div>
            <NativeInput
              id="password"
              v-model="password"
              :icon="makeIcon({ faName: 'fas fa-key' })"
              name="Password"
              title="Password"
              placeholder="correct horse battery staple"
              is-input
              :value-type="makeType({ isSecret: true })"
            />
          </div>
          <div v-if="stage == UserWizardViewStage.SIGN_UP">
            <div class="mb-1 block text-gray-700">Region</div>
            <Picker
              id="region"
              v-model="continent"
              :icon="makeIcon({ faName: 'fas fa-globe' })"
              name="Region"
              title="Region"
              is-input
              :value-type="
                makeType({ kind: TypeKind.ENUM, benchType: BenchType.CONTINENT, isList: false, isRequired: true })
              "
            />
          </div>
          <div v-if="stage == UserWizardViewStage.SIGN_UP">
            <div class="mb-1 block text-gray-700">Invite Code</div>
            <NativeInput
              id="inviteCode"
              name="Invite Code"
              title="Invite Code"
              placeholder="1234567890"
              is-input
              :icon="makeIcon({ faName: 'fas fa-user' })"
              :value-type="makeType({ kind: TypeKind.PRIMITIVE, primitiveType: PrimitiveType.INT32 })"
              :model-value="inviteCode ?? undefined"
              @update:model-value="inviteCode = $event"
            />
          </div>
        </div>
        <!-- Commands -->
        <div class="mt-7 flex flex-col gap-y-1">
          <Button
            id="submit"
            name="Submit"
            :icon="makeIcon({ faName: 'fas fa-arrow-right-from-bracket' })"
            :title="stage === UserWizardViewStage.LOG_IN ? 'Log in' : 'Sign up'"
            class="w-full"
            :is-disabled="isActive || (stage == UserWizardViewStage.SIGN_UP && !isInviteCodeValid)"
            :is-loading="isActive"
            @click="submit"
          />
          <Button
            id="switch"
            name="Switch"
            :variant="ButtonVariant.LINK"
            :icon="makeIcon({ faName: 'fas fa-shuffle' })"
            :title="stage === UserWizardViewStage.LOG_IN ? 'Sign up instead' : 'Log in instead'"
            class="mt-2 w-full"
            @click="() => switchStage()"
          />
        </div>
      </form>
      <!-- Already have User? what? -->
      <div v-else>
        <Button
          id="home"
          name="Home"
          :icon="makeIcon({ faName: 'fas fa-house' })"
          title="Go Home"
          class="mt-2 w-full"
          @click="() => fireCommandById('user.navigate.goToHome')"
        />
      </div>
    </div>
  </div>
</template>
