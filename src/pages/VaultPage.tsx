import { createSignal, onMount } from "solid-js";
import { toast } from "solid-sonner";
import { api } from "../api/tauri";
import type { VaultSummary } from "../store/types";
import { VaultSidebar } from "../components/vault/VaultSidebar";
import { VaultExplorer } from "../components/vault/VaultExplorer";
import { CreateVaultModal } from "../components/vault/CreateVaultModal";
import { UnlockModal } from "../components/vault/UnlockModal";
import styles from "./VaultPage.module.css";

export function VaultPage() {
  const [vaults, setVaults] = createSignal<VaultSummary[]>([]);
  const [activeId, setActiveId] = createSignal<string | null>(null);
  const [showCreate, setShowCreate] = createSignal(false);
  const [unlockTarget, setUnlockTarget] = createSignal<VaultSummary | null>(null);

  const loadVaults = async () => {
    try {
      const list = await api.vault.list();
      setVaults(list);
    } catch (err: any) {
      toast.error(String(err));
    }
  };

  onMount(loadVaults);

  const activeVault = () => vaults().find((v) => v.id === activeId()) ?? null;

  const handleSelectVault = (id: string) => {
    setActiveId(id);
    const vault = vaults().find((v) => v.id === id);
    if (vault && !vault.unlocked) {
      setUnlockTarget(vault);
    }
  };

  const handleUnlocked = async (vaultId: string) => {
    await loadVaults();
    setActiveId(vaultId);
    setUnlockTarget(null);
  };

  const handleVaultCreated = async (v: VaultSummary) => {
    await loadVaults();
    setActiveId(v.id);
  };

  return (
    <div class={styles.page}>
      <VaultSidebar
        vaults={vaults()}
        activeId={activeId()}
        onSelect={handleSelectVault}
        onNew={() => setShowCreate(true)}
        onVaultsChange={loadVaults}
      />

      <VaultExplorer
        vault={activeVault()}
        onVaultUpdated={loadVaults}
      />

      <CreateVaultModal
        open={showCreate()}
        onClose={() => setShowCreate(false)}
        onCreated={handleVaultCreated}
      />

      <UnlockModal
        vault={unlockTarget()}
        onClose={() => setUnlockTarget(null)}
        onUnlocked={handleUnlocked}
      />
    </div>
  );
}
