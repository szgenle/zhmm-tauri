<script setup lang="ts">
import { ref, watch } from "vue";
import PasswordStrengthBar from "./PasswordStrengthBar.vue";

const emit = defineEmits<{ (e: "confirm", password: string): void }>();

const length = ref(16);
const includeUpper = ref(true);
const includeLower = ref(true);
const includeDigits = ref(true);
const includeSymbols = ref(true);
const generated = ref("");

const UPPER = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER = "abcdefghijklmnopqrstuvwxyz";
const DIGITS = "0123456789";
const SYMBOLS = "!@#$%^&*-_=+";

function generate() {
  let chars = "";
  if (includeUpper.value) chars += UPPER;
  if (includeLower.value) chars += LOWER;
  if (includeDigits.value) chars += DIGITS;
  if (includeSymbols.value) chars += SYMBOLS;
  if (!chars) {
    // 至少保留小写
    chars = LOWER;
    includeLower.value = true;
  }
  const arr = new Uint32Array(length.value);
  crypto.getRandomValues(arr);
  generated.value = Array.from(arr)
    .map((n) => chars[n % chars.length])
    .join("");
}

function confirm() {
  if (!generated.value) generate();
  emit("confirm", generated.value);
}

// 首次自动生成
generate();

// 参数变化自动重新生成
watch([length, includeUpper, includeLower, includeDigits, includeSymbols], generate);
</script>

<template>
  <div class="random-pwd-dialog">
    <n-form label-placement="left" label-width="80">
      <n-form-item label="密码长度">
        <div style="display: flex; align-items: center; gap: 12px; width: 100%">
          <n-slider v-model:value="length" :min="8" :max="32" :step="1" style="flex: 1" />
          <n-input-number v-model:value="length" :min="8" :max="32" size="small" style="width: 80px" />
        </div>
      </n-form-item>
      <n-form-item label="字符集">
        <n-space>
          <n-checkbox v-model:checked="includeUpper">大写</n-checkbox>
          <n-checkbox v-model:checked="includeLower">小写</n-checkbox>
          <n-checkbox v-model:checked="includeDigits">数字</n-checkbox>
          <n-checkbox v-model:checked="includeSymbols">符号</n-checkbox>
        </n-space>
      </n-form-item>
      <n-form-item label="生成结果">
        <n-input :value="generated" readonly style="font-family: monospace" />
      </n-form-item>
    </n-form>
    <PasswordStrengthBar :password="generated" />
    <n-space justify="end" style="margin-top: 16px">
      <n-button @click="generate">重新生成</n-button>
      <n-button type="primary" @click="confirm">确认使用</n-button>
    </n-space>
  </div>
</template>

<style scoped>
.random-pwd-dialog {
  padding: 4px 0;
}
</style>
