import { createSignal, createMemo, onMount, onCleanup, Show } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { TbOutlineFolderOpen } from "solid-icons/tb";
import { store, setStore, refreshSources } from "../store";
import { SourceList } from "../components/sources/SourceList";
import { DestinationList } from "../components/destinations/DestinationList";
import { AddSourceModal } from "../components/sources/AddSourceModal";
import { EditSourceModal } from "../components/sources/EditSourceModal";
import { AddDestinationModal } from "../components/destinations/AddDestinationModal";
import { UpgradeModal } from "./License";
import { t } from "../i18n";
import { api } from "../api/tauri";
import type { SourceType } from "../store/types";
import styles from "./Sources.module.css";

const FREE_LIMIT = 3;

export function Sources() {
  const [showAddSource, setShowAddSource] = createSignal(false);
  const [showAddDest, setShowAddDest] = createSignal(false);
  const [showUpgrade, setShowUpgrade] = createSignal(false);
  const [showEditSource, setShowEditSource] = createSignal(false);
  const [editingSourceId, setEditingSourceId] = createSignal<string | null>(null);
  const [isDragOver, setIsDragOver] = createSignal(false);
  const [prefillPath, setPrefillPath] = createSignal<string | undefined>(undefined);
  const [prefillType, setPrefillType] = createSignal<SourceType | undefined>(undefined);

  onMount(async () => {
    refreshSources();
    const unlistenMenu = await listen("menu-open-add-source", () => handleAddSource());
    onCleanup(unlistenMenu);

    const unlistenEnter = await listen("tauri://drag-enter", () => setIsDragOver(true));
    onCleanup(unlistenEnter);

    const unlistenLeave = await listen("tauri://drag-leave", () => setIsDragOver(false));
    onCleanup(unlistenLeave);

    const unlistenDrop = await listen<{ paths: string[] }>("tauri://drag-drop", async (event) => {
      setIsDragOver(false);
      const paths = event.payload.paths;
      if (!paths || paths.length === 0) return;
      const droppedPath = paths[0];
      try {
        const pathType = await api.fs.checkPathType(droppedPath);
        setPrefillPath(droppedPath);
        setPrefillType(pathType === "Directory" ? "Directory" : "File");
        handleAddSource();
      } catch {
        setPrefillPath(droppedPath);
        setPrefillType("Directory");
        handleAddSource();
      }
    });
    onCleanup(unlistenDrop);
  });

  const activeSource = createMemo(() =>
    store.sources.find((s) => s.id === store.activeSourceId) ?? null
  );

  const editingSource = createMemo(() =>
    store.sources.find((s) => s.id === editingSourceId()) ?? null
  );

  const handleEditSource = (id: string) => {
    setEditingSourceId(id);
    setShowEditSource(true);
  };

  const handleDeleteSource = async (id: string) => {
    try {
      await api.sources.delete(id);
      if (store.activeSourceId === id) setStore("activeSourceId", null);
      await refreshSources();
    } catch (e) {
      console.error("delete source error", e);
    }
  };

  const handleAddSource = () => {
    const atLimit = store.sources.length >= FREE_LIMIT && store.licenseStatus !== "valid";
    if (atLimit) {
      setShowUpgrade(true);
    } else {
      setShowAddSource(true);
    }
  };

  return (
    <div class={`${styles.root} ${styles.rootRelative}`}>
      <Show when={isDragOver()}>
        <div class={styles.dragOverlay}>{t("drag_drop_hint")}</div>
      </Show>
      <SourceList
        sources={store.sources}
        activeId={store.activeSourceId}
        onSelect={(id) => setStore("activeSourceId", id)}
        onAdd={handleAddSource}
        onEdit={handleEditSource}
        onDelete={handleDeleteSource}
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
        onClose={() => { setShowAddSource(false); setPrefillPath(undefined); setPrefillType(undefined); }}
        onCreated={async () => { await refreshSources(); setShowAddSource(false); setPrefillPath(undefined); setPrefillType(undefined); }}
        prefillPath={prefillPath()}
        prefillType={prefillType()}
      />

      <Show when={store.activeSourceId}>
        <AddDestinationModal
          open={showAddDest()}
          onClose={() => setShowAddDest(false)}
          sourceId={store.activeSourceId!}
          onCreated={async () => { await refreshSources(); setShowAddDest(false); }}
        />
      </Show>

      <EditSourceModal
        open={showEditSource()}
        onClose={() => setShowEditSource(false)}
        source={editingSource()}
        onUpdated={async () => { await refreshSources(); setShowEditSource(false); }}
      />

      <UpgradeModal
        open={showUpgrade()}
        onClose={() => setShowUpgrade(false)}
        sourceCount={store.sources.length}
      />
    </div>
  );
}
