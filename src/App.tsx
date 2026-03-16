import { createEffect, createSignal, onMount, onCleanup, Switch, Match, ErrorBoundary } from "solid-js";
import { listen, emit } from "@tauri-apps/api/event";
import { Toaster } from "solid-sonner";
import { api } from "./api/tauri";
import { Layout } from "./components/layout/Layout";
import { Dashboard } from "./pages/Dashboard";
import { Sources } from "./pages/Sources";
import { Logs } from "./pages/Logs";
import { Settings } from "./pages/Settings";
import { LicensePage } from "./pages/LicensePage";
import { VaultPage } from "./pages/VaultPage";
import { AboutModal } from "./components/ui/AboutModal";
import { OnboardingModal } from "./components/ui/OnboardingModal";
import { ErrorFallback } from "./components/ui/ErrorFallback";
import { BUY_URL } from "./constants";
import { store, setStore, initStore, initLicense } from "./store";
import "./styles/globals.css";

export function App() {
  const [showAbout, setShowAbout] = createSignal(false);
  const [showOnboarding, setShowOnboarding] = createSignal(false);

  onMount(async () => {
    await initStore();
    initLicense();

    // Show onboarding on first run
    const done = await api.settings.getValue("onboarding_done").catch(() => null);
    if (done !== "true") setShowOnboarding(true);

    // Keyboard shortcuts
    const handleKeyDown = (e: KeyboardEvent) => {
      const meta = e.metaKey || e.ctrlKey;
      if (!meta) return;
      if (e.key === "n" || e.key === "N") {
        e.preventDefault();
        setStore("activePage", "sources");
        emit("menu-open-add-source").catch(() => {});
      }
      if (e.key === "r" || e.key === "R") {
        e.preventDefault();
        for (const src of store.sources) {
          api.jobs.runSourceNow(src.id).catch(() => {});
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    onCleanup(() => window.removeEventListener("keydown", handleKeyDown));

    const unlisteners = await Promise.all([
      // About modal
      listen("show-about", () => setShowAbout(true)),

      // Navigation from native menu
      listen<string>("menu-navigate", (e) => {
        setStore("activePage", e.payload as any);
      }),

      // Sidebar toggle from native menu (Cmd+\)
      listen("menu-toggle-sidebar", () => {
        setStore("sidebarCollapsed", !store.sidebarCollapsed);
      }),

      // Run all sources from native menu (Cmd+Shift+R)
      listen("menu-run-all", async () => {
        for (const src of store.sources) {
          api.jobs.runSourceNow(src.id).catch(() => {});
        }
      }),

      // Open buy URL from About section in native menu
      listen("menu-open-buy-url", async () => {
        try {
          const { open } = await import("@tauri-apps/plugin-shell");
          await open(BUY_URL);
        } catch {
          window.open(BUY_URL, "_blank");
        }
      }),
    ]);

    onCleanup(() => unlisteners.forEach((fn) => fn()));
  });

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
    <ErrorBoundary fallback={(err, reset) => <ErrorFallback error={err} reset={reset} />}>
      <Layout>
        <Switch>
          <Match when={store.activePage === "dashboard"}><Dashboard /></Match>
          <Match when={store.activePage === "sources"}><Sources /></Match>
          <Match when={store.activePage === "logs"}><Logs /></Match>
          <Match when={store.activePage === "settings"}><Settings /></Match>
          <Match when={store.activePage === "license"}><LicensePage /></Match>
          <Match when={store.activePage === "vault"}><VaultPage /></Match>
        </Switch>
      </Layout>
      <AboutModal open={showAbout()} onClose={() => setShowAbout(false)} />
      <OnboardingModal open={showOnboarding()} onClose={() => setShowOnboarding(false)} />
      <Toaster
        position="bottom-right"
        richColors
        theme={store.settings?.theme ?? "dark"}
        duration={3500}
      />
    </ErrorBoundary>
  );
}
