import { createSignal, createMemo, onMount, onCleanup, Show } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { TbOutlineFolderOpen } from "solid-icons/tb";
import { store, setStore, refreshSources } from "../store";
import { SourceList } from "../components/sources/SourceList";
import { DestinationList } from "../components/destinations/DestinationList";
import { AddSourceModal } from "../components/sources/AddSourceModal";
import { AddDestinationModal } from "../components/destinations/AddDestinationModal";
import { UpgradeModal } from "./License";
import { t } from "../i18n";
import styles from "./Sources.module.css";

const FREE_LIMIT = 3;

export function Sources() {
  const [showAddSource, setShowAddSource] = createSignal(false);
  const [showAddDest, setShowAddDest] = createSignal(false);
  const [showUpgrade, setShowUpgrade] = createSignal(false);

  onMount(async () => {
    refreshSources();
    const unlisten = await listen("menu-open-add-source", () => handleAddSource());
    onCleanup(unlisten);
  });

  const activeSource = createMemo(() =>
    store.sources.find((s) => s.id === store.activeSourceId) ?? null
  );

  const handleAddSource = () => {
    const atLimit = store.sources.length >= FREE_LIMIT && store.licenseStatus !== "valid";
    if (atLimit) {
      setShowUpgrade(true);
    } else {
      setShowAddSource(true);
    }
  };

  return (
    <div class={styles.root}>
      <SourceList
        sources={store.sources}
        activeId={store.activeSourceId}
        onSelect={(id) => setStore("activeSourceId", id)}
        onAdd={handleAddSource}
      />

      <Show when={activeSource()} fallback={
        <div class={styles.placeholder}>
          <div class={styles.placeholderInner}>
            <div class={styles.placeholderIcon}><TbOutlineFolderOpen size={40} /></div>
            <div>{t("src_select_hint")}</div>
            <div class={styles.placeholderHint}>{t("src_select_hint2")}</div>
          </div>
        </div>
      }>
        {(source) => (
          <DestinationList
            source={source()}
            runningJobs={store.runningJobs}
            onAddDestination={() => setShowAddDest(true)}
            onRefresh={refreshSources}
          />
        )}
      </Show>

      <AddSourceModal
        open={showAddSource()}
        onClose={() => setShowAddSource(false)}
        onCreated={async () => { await refreshSources(); setShowAddSource(false); }}
      />

      <Show when={store.activeSourceId}>
        <AddDestinationModal
          open={showAddDest()}
          onClose={() => setShowAddDest(false)}
          sourceId={store.activeSourceId!}
          onCreated={async () => { await refreshSources(); setShowAddDest(false); }}
        />
      </Show>

      <UpgradeModal
        open={showUpgrade()}
        onClose={() => setShowUpgrade(false)}
        sourceCount={store.sources.length}
      />
    </div>
  );
}
