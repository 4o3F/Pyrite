import { createApp } from "vue";
import "./main.css";
import App from "./App.vue";
import router from "./router";
import { MotionPlugin } from "@vueuse/motion";

let app = createApp(App);
app.use(router);
app.use(MotionPlugin);

app.mount("#app");
