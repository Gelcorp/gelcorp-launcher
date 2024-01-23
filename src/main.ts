import "./styles.css";
import Main from "./App.svelte";

document.addEventListener("contextmenu", event => {
  event.preventDefault();
})

const app = new Main({
  target: document.getElementById("app") as Element,
});

export default app;
