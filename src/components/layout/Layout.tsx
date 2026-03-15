import { JSX } from "solid-js";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";
import styles from "./Layout.module.css";

export function Layout(props: { children: JSX.Element }) {
  return (
    <div class={styles.root}>
      <TopBar />
      <div class={styles.body}>
        <Sidebar />
        <main class={styles.main}>{props.children}</main>
      </div>
    </div>
  );
}
