import { createSignal, Show } from "solid-js";
import { toast } from "solid-sonner";
import { TbOutlineAlertTriangle } from "solid-icons/tb";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { Toggle } from "../ui/Toggle";
import { SchedulePicker } from "../schedule/SchedulePicker";
import { UpgradeModal } from "../../pages/License";
import { t } from "../../i18n";
import { api } from "../../api/tauri";
import { store } from "../../store";
import type { ScheduleType, RetentionPolicy, DestinationType, S3Config, SftpConfig, OAuthConfig } from "../../store/types";
import styles from "./AddDestinationModal.module.css";

interface Props {
  open: boolean;
  onClose: () => void;
  sourceId: string;
  onCreated: () => void;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function AddDestinationModal(props: Props) {
  // Destination type
  const [destType, setDestType] = createSignal<DestinationType>("Local");

  // Local fields
  const [destPath, setDestPath] = createSignal("");
  const [availBytes, setAvailBytes] = createSignal<number | null>(null);

  // Cloud fields
  const [cloudProvider, setCloudProvider] = createSignal<"S3" | "R2" | "Sftp" | "OAuth">("S3");
  const [bucket, setBucket] = createSignal("");
  const [region, setRegion] = createSignal("us-east-1");
  const [accessKeyId, setAccessKeyId] = createSignal("");
  const [secretAccessKey, setSecretAccessKey] = createSignal("");
  const [accountId, setAccountId] = createSignal(""); // R2 account ID
  const [prefix, setPrefix] = createSignal("");
  // SFTP fields
  const [sftpHost, setSftpHost] = createSignal("");
  const [sftpPort, setSftpPort] = createSignal(22);
  const [sftpUsername, setSftpUsername] = createSignal("");
  const [sftpAuthType, setSftpAuthType] = createSignal<"password" | "key">("password");
  const [sftpPassword, setSftpPassword] = createSignal("");
  const [sftpKeyPath, setSftpKeyPath] = createSignal("");
  const [sftpRemotePath, setSftpRemotePath] = createSignal("/");
  // OAuth fields
  const [oauthProvider, setOauthProvider] = createSignal<"onedrive" | "gdrive">("onedrive");
  const [oauthClientId, setOauthClientId] = createSignal("");
  const [oauthFolderPath, setOauthFolderPath] = createSignal("/ShadowVault");
  const [oauthConfig, setOauthConfig] = createSignal<OAuthConfig | null>(null);
  const [oauthStatus, setOauthStatus] = createSignal<"idle" | "waiting" | "done" | "error">("idle");
  const [oauthError, setOauthError] = createSignal("");
  const [testing, setTesting] = createSignal(false);

  // Common fields
  const [schedule, setSchedule] = createSignal<ScheduleType>({ type: "Interval", value: { minutes: 60 } });
  const [maxVersions, setMaxVersions] = createSignal(10);
  const [naming, setNaming] = createSignal<"Timestamp" | "Index" | "Overwrite">("Timestamp");
  const [exclusionsText, setExclusionsText] = createSignal("");
  const [incremental, setIncremental] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [showUpgrade, setShowUpgrade] = createSignal(false);
  const isLicensed = () => store.licenseStatus === "valid";
  const LOW_SPACE_THRESHOLD = 500 * 1024 * 1024;

  const retention = (): RetentionPolicy => ({ max_versions: maxVersions(), naming: naming() });

  const r2Endpoint = () =>
    accountId().trim() ? `https://${accountId().trim()}.r2.cloudflarestorage.com` : undefined;

  const reset = () => {
    setDestType("Local");
    setDestPath(""); setAvailBytes(null);
    setCloudProvider("S3"); setBucket(""); setRegion("us-east-1");
    setAccessKeyId(""); setSecretAccessKey(""); setAccountId(""); setPrefix("");
    setSftpHost(""); setSftpPort(22); setSftpUsername(""); setSftpAuthType("password");
    setSftpPassword(""); setSftpKeyPath(""); setSftpRemotePath("/");
    setOauthProvider("onedrive"); setOauthClientId(""); setOauthFolderPath("/ShadowVault");
    setOauthConfig(null); setOauthStatus("idle"); setOauthError("");
    setSchedule({ type: "Interval", value: { minutes: 60 } });
    setMaxVersions(10); setNaming("Timestamp"); setExclusionsText(""); setIncremental(false);
    setSaving(false);
  };

  const handleClose = () => { reset(); props.onClose(); };

  const checkDiskSpace = async (path: string) => {
    if (!path.trim()) { setAvailBytes(null); return; }
    try {
      const info = await api.fs.getDiskInfo(path);
      setAvailBytes(info.available_bytes);
    } catch { setAvailBytes(null); }
  };

  const pickDest = async () => {
    try {
      const picked = await api.fs.pickDirectory();
      if (picked) { setDestPath(picked); await checkDiskSpace(picked); }
    } catch { toast.error(t("add_dest_pick_err")); }
  };

  const handleTestConnection = async () => {
    const prov = cloudProvider();
    if (prov === "Sftp") {
      if (!sftpHost().trim() || !sftpUsername().trim()) {
        toast.error(t("sftp_fields_required")); return;
      }
      setTesting(true);
      try {
        await api.cloud.testSftpConnection(
          sftpHost().trim(), sftpPort(),
          sftpUsername().trim(), sftpAuthType(),
          sftpAuthType() === "password" ? sftpPassword().trim() : null,
          sftpAuthType() === "key" ? sftpKeyPath().trim() : null,
          sftpRemotePath().trim() || "/",
        );
        toast.success(t("cloud_connection_ok"));
      } catch (e: any) {
        toast.error(e?.message ?? t("cloud_connection_err"));
      } finally { setTesting(false); }
      return;
    }
    if (!bucket().trim() || !accessKeyId().trim() || !secretAccessKey().trim()) {
      toast.error(t("cloud_fields_required"));
      return;
    }
    if (prov === "R2" && !accountId().trim()) {
      toast.error(t("cloud_r2_account_required"));
      return;
    }
    setTesting(true);
    try {
      await api.cloud.testConnection(
        prov,
        bucket().trim(),
        prov === "R2" ? "auto" : region().trim(),
        accessKeyId().trim(),
        secretAccessKey().trim(),
        prov === "R2" ? (r2Endpoint() ?? null) : null,
        prefix().trim(),
      );
      toast.success(t("cloud_connection_ok"));
    } catch (e: any) {
      toast.error(e?.message ?? t("cloud_connection_err"));
    } finally { setTesting(false); }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const exclusions = exclusionsText().split("\n").map(s => s.trim()).filter(Boolean);

      if (destType() === "Local") {
        if (!destPath().trim()) { toast.error(t("add_dest_path_req")); setSaving(false); return; }
        await api.destinations.add(props.sourceId, destPath(), schedule(), retention(), exclusions, incremental(), "Local", null, null);
      } else if (destType() === "Sftp") {
        if (!sftpHost().trim() || !sftpUsername().trim()) { toast.error(t("sftp_fields_required")); setSaving(false); return; }
        const sftpConfig: SftpConfig = {
          host: sftpHost().trim(),
          port: sftpPort(),
          username: sftpUsername().trim(),
          auth_type: sftpAuthType(),
          password: sftpAuthType() === "password" ? sftpPassword().trim() : undefined,
          private_key: sftpAuthType() === "key" ? sftpKeyPath().trim() : undefined,
          remote_path: sftpRemotePath().trim() || "/",
        };
        const displayPath = `sftp://${sftpHost().trim()}${sftpRemotePath().trim() || "/"}`;
        await api.destinations.add(props.sourceId, displayPath, schedule(), retention(), exclusions, incremental(), "Sftp", null, sftpConfig, null);
      } else if (destType() === "OneDrive" || destType() === "GoogleDrive") {
        if (!oauthConfig()) { toast.error(t("oauth_not_connected")); setSaving(false); return; }
        const displayPath = `${oauthProvider()}://${oauthFolderPath().trim() || "/ShadowVault"}`;
        await api.destinations.add(props.sourceId, displayPath, schedule(), retention(), exclusions, incremental(), destType() as DestinationType, null, null, oauthConfig());
      } else {
        const prov = cloudProvider() as "S3" | "R2";
        if (!bucket().trim()) { toast.error(t("cloud_bucket_required")); setSaving(false); return; }
        if (!accessKeyId().trim() || !secretAccessKey().trim()) { toast.error(t("cloud_keys_required")); setSaving(false); return; }
        if (prov === "R2" && !accountId().trim()) { toast.error(t("cloud_r2_account_required")); setSaving(false); return; }

        const cloudConfig: S3Config = {
          provider: prov,
          bucket: bucket().trim(),
          region: prov === "R2" ? "auto" : region().trim(),
          access_key_id: accessKeyId().trim(),
          secret_access_key: secretAccessKey().trim(),
          endpoint_url: prov === "R2" ? r2Endpoint() : undefined,
          prefix: prefix().trim(),
        };
        const displayPath = `${prov.toLowerCase()}://${bucket().trim()}/${prefix().trim()}`;
        await api.destinations.add(props.sourceId, displayPath, schedule(), retention(), exclusions, incremental(), prov as DestinationType, cloudConfig, null);
      }

      props.onCreated();
      handleClose();
    } catch (e: any) {
      toast.error(e?.message ?? t("add_dest_save_err"));
    } finally { setSaving(false); }
  };

  return (
    <>
    <Modal
      open={props.open}
      closeOnBackdrop={false}
      onClose={handleClose}
      title={t("add_dest_title")}
      footer={
        <div class={styles.footerRow}>
          <Button variant="ghost" onClick={handleClose}>{t("btn_cancel")}</Button>
          <Button onClick={handleSave} disabled={saving()}>
            {saving() ? t("btn_saving") : t("btn_save")}
          </Button>
        </div>
      }
    >
      {/* Destination type tabs */}
      <div class={styles.typeTabs}>
        <button
          class={styles.typeTab}
          data-active={String(destType() === "Local")}
          onClick={() => setDestType("Local")}
        >
          {t("dest_type_local")}
        </button>
        <button
          class={styles.typeTab}
          data-active={String(destType() === "S3" || destType() === "R2")}
          onClick={() => { setDestType("S3"); setCloudProvider("S3"); }}
        >
          {t("dest_type_cloud")}
        </button>
        <button
          class={styles.typeTab}
          data-active={String(destType() === "Sftp")}
          onClick={() => { setDestType("Sftp"); setCloudProvider("Sftp"); }}
        >
          {t("dest_type_sftp")}
        </button>
        <button
          class={styles.typeTab}
          data-active={String(destType() === "OneDrive" || destType() === "GoogleDrive")}
          onClick={() => { setDestType("OneDrive"); setOauthProvider("onedrive"); setCloudProvider("OAuth"); }}
        >
          {t("dest_type_oauth")}
        </button>
      </div>

      {/* Local fields */}
      <Show when={destType() === "Local"}>
        <div class={styles.field}>
          <label class={styles.label}>{t("add_dest_folder")}</label>
          <div class={styles.inputRow}>
            <input
              class={`${styles.input} ${styles.inputFlex}`}
              type="text"
              placeholder="/backup/target"
              value={destPath()}
              onInput={(e) => { setDestPath(e.currentTarget.value); checkDiskSpace(e.currentTarget.value); }}
            />
            <Button variant="ghost" size="sm" onClick={pickDest}>{t("btn_browse")}</Button>
          </div>
          <Show when={availBytes() !== null}>
            <div class={styles.diskInfo} data-low={String((availBytes() ?? 0) < LOW_SPACE_THRESHOLD)}>
              <Show when={(availBytes() ?? 0) < LOW_SPACE_THRESHOLD}>
                <span class={styles.lowSpaceLabel}><TbOutlineAlertTriangle size={13} /> {t("add_dest_low_space")} </span>
              </Show>
              {t("add_dest_avail_space")} {formatBytes(availBytes()!)}
            </div>
          </Show>
        </div>
      </Show>

      {/* Cloud fields (S3/R2) */}
      <Show when={destType() === "S3" || destType() === "R2"}>
        <div class={styles.field}>
          <label class={styles.label}>{t("cloud_provider")}</label>
          <select
            class={styles.input}
            value={cloudProvider()}
            onChange={(e) => {
              const v = e.currentTarget.value as "S3" | "R2";
              setCloudProvider(v);
              setDestType(v);
              if (v === "S3") setRegion("us-east-1");
            }}
          >
            <option value="S3">AWS S3</option>
            <option value="R2">Cloudflare R2</option>
          </select>
        </div>

        <div class={styles.field}>
          <label class={styles.label}>{t("cloud_bucket")}</label>
          <input class={styles.input} type="text" placeholder="my-backup-bucket"
            value={bucket()} onInput={e => setBucket(e.currentTarget.value)} />
        </div>

        <Show when={cloudProvider() === "R2"}>
          <div class={styles.field}>
            <label class={styles.label}>{t("cloud_r2_account_id")}</label>
            <input class={styles.input} type="text" placeholder="abc123def456..."
              value={accountId()} onInput={e => setAccountId(e.currentTarget.value)} />
            <div class={styles.hint}>{t("cloud_r2_account_hint")}</div>
          </div>
        </Show>

        <Show when={cloudProvider() === "S3"}>
          <div class={styles.field}>
            <label class={styles.label}>{t("cloud_region")}</label>
            <input class={styles.input} type="text" placeholder="us-east-1"
              value={region()} onInput={e => setRegion(e.currentTarget.value)} />
          </div>
        </Show>

        <div class={styles.field}>
          <label class={styles.label}>{t("cloud_access_key_id")}</label>
          <input class={styles.input} type="text" placeholder="AKIAIOSFODNN7EXAMPLE"
            value={accessKeyId()} onInput={e => setAccessKeyId(e.currentTarget.value)} />
        </div>

        <div class={styles.field}>
          <label class={styles.label}>{t("cloud_secret_access_key")}</label>
          <input class={styles.input} type="password" placeholder="••••••••••••••••••••"
            value={secretAccessKey()} onInput={e => setSecretAccessKey(e.currentTarget.value)} />
        </div>

        <div class={styles.field}>
          <label class={styles.label}>{t("cloud_prefix")}</label>
          <input class={styles.input} type="text" placeholder="backups/myapp"
            value={prefix()} onInput={e => setPrefix(e.currentTarget.value)} />
          <div class={styles.hint}>{t("cloud_prefix_hint")}</div>
        </div>

        <div class={styles.field}>
          <Button variant="ghost" size="sm" onClick={handleTestConnection} disabled={testing()}>
            {testing() ? t("cloud_testing") : t("cloud_test_btn")}
          </Button>
        </div>
      </Show>

      {/* SFTP fields */}
      <Show when={destType() === "Sftp"}>
        <div class={styles.field}>
          <label class={styles.label}>{t("sftp_host")}</label>
          <input class={styles.input} type="text" placeholder="ssh.example.com"
            value={sftpHost()} onInput={e => setSftpHost(e.currentTarget.value)} />
        </div>

        <div class={styles.retentionRow}>
          <div class={styles.retentionCol} style={{ flex: "2" }}>
            <label class={styles.label}>{t("sftp_username")}</label>
            <input class={styles.input} type="text" placeholder="ubuntu"
              value={sftpUsername()} onInput={e => setSftpUsername(e.currentTarget.value)} />
          </div>
          <div class={styles.retentionCol} style={{ flex: "1" }}>
            <label class={styles.label}>{t("sftp_port")}</label>
            <input class={styles.input} type="number" min={1} max={65535} value={sftpPort()}
              onInput={e => setSftpPort(parseInt(e.currentTarget.value) || 22)} />
          </div>
        </div>

        <div class={styles.field}>
          <label class={styles.label}>{t("sftp_auth_type")}</label>
          <select class={styles.input} value={sftpAuthType()}
            onChange={e => setSftpAuthType(e.currentTarget.value as "password" | "key")}>
            <option value="password">{t("sftp_auth_password")}</option>
            <option value="key">{t("sftp_auth_key")}</option>
          </select>
        </div>

        <Show when={sftpAuthType() === "password"}>
          <div class={styles.field}>
            <label class={styles.label}>{t("sftp_password")}</label>
            <input class={styles.input} type="password" placeholder="••••••••"
              value={sftpPassword()} onInput={e => setSftpPassword(e.currentTarget.value)} />
          </div>
        </Show>

        <Show when={sftpAuthType() === "key"}>
          <div class={styles.field}>
            <label class={styles.label}>{t("sftp_key_path")}</label>
            <input class={styles.input} type="text" placeholder={t("sftp_key_path_ph")}
              value={sftpKeyPath()} onInput={e => setSftpKeyPath(e.currentTarget.value)} />
          </div>
        </Show>

        <div class={styles.field}>
          <label class={styles.label}>{t("sftp_remote_path")}</label>
          <input class={styles.input} type="text" placeholder={t("sftp_remote_path_ph")}
            value={sftpRemotePath()} onInput={e => setSftpRemotePath(e.currentTarget.value)} />
        </div>

        <div class={styles.field}>
          <Button variant="ghost" size="sm" onClick={handleTestConnection} disabled={testing()}>
            {testing() ? t("cloud_testing") : t("cloud_test_btn")}
          </Button>
        </div>
      </Show>

      {/* OAuth fields (OneDrive / Google Drive) */}
      <Show when={destType() === "OneDrive" || destType() === "GoogleDrive"}>
        <div class={styles.field}>
          <label class={styles.label}>{t("oauth_provider")}</label>
          <select class={styles.input} value={oauthProvider()}
            onChange={e => {
              const v = e.currentTarget.value as "onedrive" | "gdrive";
              setOauthProvider(v);
              setDestType(v === "onedrive" ? "OneDrive" : "GoogleDrive");
              setOauthConfig(null); setOauthStatus("idle");
            }}>
            <option value="onedrive">Microsoft OneDrive</option>
            <option value="gdrive">Google Drive</option>
          </select>
        </div>

        <div class={styles.field}>
          <label class={styles.label}>{t("oauth_client_id")}</label>
          <input class={styles.input} type="text" placeholder={t("oauth_client_id_ph")}
            value={oauthClientId()} onInput={e => setOauthClientId(e.currentTarget.value)} />
          <div class={styles.hint}>{t("oauth_client_id_hint")}</div>
        </div>

        <div class={styles.field}>
          <label class={styles.label}>{t("oauth_folder_path")}</label>
          <input class={styles.input} type="text" placeholder="/ShadowVault/backups"
            value={oauthFolderPath()} onInput={e => setOauthFolderPath(e.currentTarget.value)} />
        </div>

        <div class={styles.field}>
          <Show when={oauthStatus() === "done"}>
            <div class={styles.hint} style={{ color: "var(--color-success, #4ade80)" }}>
              {t("oauth_connected_ok")}
            </div>
          </Show>
          <Show when={oauthStatus() === "error"}>
            <div class={styles.hint} style={{ color: "var(--color-danger, #f87171)" }}>
              {oauthError()}
            </div>
          </Show>
          <div style={{ display: "flex", gap: "8px" }}>
            <Button variant="ghost" size="sm"
              disabled={oauthStatus() === "waiting"}
              onClick={async () => {
                if (!oauthClientId().trim()) { toast.error(t("oauth_client_id_required")); return; }
                setOauthStatus("waiting"); setOauthError("");
                try {
                  const cfg = await api.oauth.runFlow(oauthProvider(), oauthClientId().trim(), oauthFolderPath().trim() || "/ShadowVault");
                  setOauthConfig(cfg); setOauthStatus("done");
                  toast.success(t("oauth_connected_ok"));
                } catch (e: any) {
                  setOauthStatus("error");
                  setOauthError(e?.message ?? t("oauth_connect_err"));
                  toast.error(e?.message ?? t("oauth_connect_err"));
                }
              }}>
              {oauthStatus() === "waiting" ? t("oauth_waiting") : t("oauth_connect_btn")}
            </Button>
            <Show when={oauthStatus() === "done"}>
              <Button variant="ghost" size="sm" disabled={testing()}
                onClick={async () => {
                  if (!oauthConfig()) return;
                  setTesting(true);
                  try {
                    await api.oauth.testConnection(oauthConfig()!);
                    toast.success(t("cloud_connection_ok"));
                  } catch (e: any) {
                    toast.error(e?.message ?? t("cloud_connection_err"));
                  } finally { setTesting(false); }
                }}>
                {testing() ? t("cloud_testing") : t("cloud_test_btn")}
              </Button>
            </Show>
          </div>
        </div>
      </Show>

      <div class={styles.field}>
        <label class={styles.label}>{t("add_dest_schedule")}</label>
        <div class={styles.scheduleBox}>
          <SchedulePicker
            value={schedule()}
            onChange={setSchedule}
            isLicensed={isLicensed()}
            onProRequired={() => setShowUpgrade(true)}
          />
        </div>
      </div>

      <div class={styles.retentionRow}>
        <div class={styles.retentionCol}>
          <label class={styles.label}>{t("add_dest_max_ver")}</label>
          <input class={styles.input} type="number" min={1} max={999} value={maxVersions()}
            onInput={(e) => setMaxVersions(parseInt(e.currentTarget.value) || 10)} />
        </div>
        <div class={styles.retentionCol}>
          <label class={styles.label}>{t("add_dest_naming")}</label>
          <select class={styles.input} value={naming()} onChange={(e) => setNaming(e.currentTarget.value as any)}>
            <option value="Timestamp">{t("naming_timestamp")}</option>
            <option value="Index">{t("naming_index")}</option>
            <option value="Overwrite">{t("naming_overwrite")}</option>
          </select>
        </div>
      </div>

      <div class={styles.field}>
        <Toggle value={incremental()} onChange={setIncremental} label={t("add_dest_incremental")} />
        <div class={styles.hint}>{t("add_dest_incremental_desc")}</div>
      </div>

      <div class={styles.field}>
        <label class={styles.label}>{t("add_dest_exclusions")}</label>
        <textarea
          class={styles.textarea}
          rows={4}
          placeholder={t("add_dest_exclusions_ph")}
          value={exclusionsText()}
          onInput={(e) => setExclusionsText(e.currentTarget.value)}
          spellcheck={false}
        />
        <div class={styles.hint}>{t("add_dest_exclusions_hint")}</div>
      </div>
    </Modal>

    <UpgradeModal
      open={showUpgrade()}
      onClose={() => setShowUpgrade(false)}
      sourceCount={0}
      subtitle={t("pro_schedule_sub")}
    />
    </>
  );
}
