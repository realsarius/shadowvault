import { store, setStore } from "../../store";
import styles from "./Sidebar.module.css";

const items = [
  { id: "dashboard", label: "Genel Bakış", icon: "⊞" },
  { id: "sources",   label: "Kaynaklar",   icon: "📁" },
  { id: "logs",      label: "Loglar",       icon: "📋" },
  { id: "settings",  label: "Ayarlar",      icon: "⚙" },
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
          {item.icon} {item.label}
        </button>
      ))}
    </nav>
  );
}
