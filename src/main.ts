import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import { migrateZhmmToAjot } from "./utils/storageMigration";
import "./styles/global.css";

// v2.0 品牌切换：把 zhmm_* / zhmm:* 前缀的本地存储一次性迁移到 ajot 前缀
migrateZhmmToAjot();

createApp(App).use(router).mount("#app");
