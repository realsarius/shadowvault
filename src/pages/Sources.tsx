import { createSignal, createMemo, onMount, Show } from "solid-js";
import { store, setStore, refreshSources } from "../store";
import { SourceList } from "../components/sources/SourceList";
import { DestinationList } from "../components/destinations/DestinationList";
import { AddSourceModal } from "../components/sources/AddSourceModal";
import { AddDestinationModal } from "../components/destinations/AddDestinationModal";
import { t } from "../i18n";
import styles from "./Sources.module.css";

export function Sources() {
  const [showAddSource, setShowAddSource] = createSignal(false);
  const [showAddDest, setShowAddDest] = createSignal(false);

  onMount(() => refreshSources());

  const activeSource = createMemo(() =>
    store.sources.find((s) => s.id === store.activeSourceId) ?? null
  );

  return (
    <div class={styles.root}>
      <SourceList
        sources={store.sources}
        activeId={store.activeSourceId}
        onSelect={(id) => setStore("activeSourceId", id)}
        onAdd={() => setShowAddSource(true)}
      />

      <Show when={activeSource()} fallback={
        <div class={styles.placeholder}>
          <div class={styles.placeholderInner}>
            <div class={styles.placeholderIcon}>📁</div>
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
    </div>
  );
}
