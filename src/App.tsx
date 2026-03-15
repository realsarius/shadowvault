import { createEffect, onMount, Switch, Match } from "solid-js";
import { Layout } from "./components/layout/Layout";
import { Dashboard } from "./pages/Dashboard";
import { Sources } from "./pages/Sources";
import { Logs } from "./pages/Logs";
import { Settings } from "./pages/Settings";
import { store, initStore } from "./store";
import "./styles/globals.css";

export function App() {
  onMount(() => initStore());

  createEffect(() => {
    const theme = store.settings?.theme ?? "dark";
    const root = document.documentElement;
    if (theme === "system") {
      const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
      root.setAttribute("data-theme", prefersDark ? "dark" : "light");
    } else {
      root.setAttribute("data-theme", theme);
    }
  });

  return (
    <Layout>
      <Switch>
        <Match when={store.activePage === "dashboard"}><Dashboard /></Match>
        <Match when={store.activePage === "sources"}><Sources /></Match>
        <Match when={store.activePage === "logs"}><Logs /></Match>
        <Match when={store.activePage === "settings"}><Settings /></Match>
      </Switch>
    </Layout>
  );
}
