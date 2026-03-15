import { store, setStore } from "../../store";
import { t } from "../../i18n";
import styles from "./Sidebar.module.css";

const items = [
  { id: "dashboard", labelKey: "nav_dashboard" as const, icon: "⊞" },
  { id: "sources",   labelKey: "nav_sources" as const,   icon: "📁" },
  { id: "logs",      labelKey: "nav_logs" as const,      icon: "📋" },
  { id: "settings",  labelKey: "nav_settings" as const,  icon: "⚙" },
] as const;

export function Sidebar() {
  return (
    <nav class={styles.nav}>
      {items.map((item) => (
        <button
          class={styles.item}
          data-active={String(store.activePage === item.id)}
          onClick={() => setStore("activePage", item.id)}
        >
          {item.icon} {t(item.labelKey)}
        </button>
      ))}
    </nav>
  );
}
