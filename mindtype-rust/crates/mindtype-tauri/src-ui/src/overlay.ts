import "./styles/overlay.css";
import OverlayApp from "./lib/views/OverlayApp.svelte";
import { mount } from "svelte";

const app = mount(OverlayApp, {
  target: document.getElementById("overlay")!,
});

export default app;
