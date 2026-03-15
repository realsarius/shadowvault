import { JSX } from "solid-js";
import {
  TbOutlineLayoutDashboard,
  TbOutlineFolders,
  TbOutlineClipboardList,
  TbOutlineSettings2,
  TbOutlineCrown,
  TbOutlineMenu,
  TbOutlineChevronLeft,
} from "solid-icons/tb";
import { store, setStore } from "../../store";
import { t } from "../../i18n";
import styles from "./Sidebar.module.css";

type Page = "dashboard" | "sources" | "logs" | "settings" | "license";

interface NavItem {
  id: Page;
  labelKey: "nav_dashboard" | "nav_sources" | "nav_logs" | "nav_license" | "nav_settings";
  icon: (props: { size: number }) => JSX.Element;
}

const items: NavItem[] = [
  { id: "dashboard", labelKey: "nav_dashboard", icon: (p) => <TbOutlineLayoutDashboard size={p.size} /> },
  { id: "sources",   labelKey: "nav_sources",   icon: (p) => <TbOutlineFolders size={p.size} /> },
  { id: "logs",      labelKey: "nav_logs",      icon: (p) => <TbOutlineClipboardList size={p.size} /> },
  { id: "license",   labelKey: "nav_license",   icon: (p) => <TbOutlineCrown size={p.size} /> },
  { id: "settings",  labelKey: "nav_settings",  icon: (p) => <TbOutlineSettings2 size={p.size} /> },
];

export function Sidebar() {
  const collapsed = () => store.sidebarCollapsed;

  return (
    <nav class={styles.nav} data-collapsed={String(collapsed())}>
      <div class={styles.navItems}>
        {items.map((item) => (
          <button
            class={styles.item}
            data-active={String(store.activePage === item.id)}
            data-collapsed={String(collapsed())}
            onClick={() => setStore("activePage", item.id)}
            title={collapsed() ? t(item.labelKey) : undefined}
          >
            <span class={styles.itemIcon}>{item.icon({ size: 17 })}</span>
            {!collapsed() && <span class={styles.itemLabel}>{t(item.labelKey)}</span>}
          </button>
        ))}
      </div>

      <button
        class={styles.collapseBtn}
        onClick={() => setStore("sidebarCollapsed", !collapsed())}
        title={collapsed() ? t("nav_expand") : t("nav_collapse")}
      >
        {collapsed()
          ? <TbOutlineMenu size={17} />
          : <TbOutlineChevronLeft size={17} />}
      </button>
    </nav>
  );
}
