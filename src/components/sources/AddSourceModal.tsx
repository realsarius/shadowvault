import { createSignal, createEffect, Show } from "solid-js";
import { toast } from "solid-sonner";
import { TbOutlineAlertTriangle } from "solid-icons/tb";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { SchedulePicker } from "../schedule/SchedulePicker";
import { UpgradeModal } from "../../pages/License";
import { t } from "../../i18n";
import { api } from "../../api/tauri";
import { store } from "../../store";
import type { ScheduleType, RetentionPolicy, SourceType, DestinationType, S3Config, SftpConfig, OAuthConfig, WebDavConfig } from "../../store/types";
import styles from "./AddSourceModal.module.css";

interface Props {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
  prefillPath?: string;
  prefillType?: SourceType;
}

export function AddSourceModal(props: Props) {
  const [step, setStep] = createSignal(1);
  // Source fields
  const [name, setName] = createSignal("");
  const [sourcePath, setSourcePath] = createSignal("");
  const [sourceType, setSourceType] = createSignal<SourceType>("Directory");

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
  const [accountId, setAccountId] = createSignal("");
  const [prefix, setPrefix] = createSignal("");

  // SFTP fields
  const [sftpHost, setSftpHost] = createSignal("");
  const [sftpPort, setSftpPort] = createSignal(22);
  const [sftpUsername, setSftpUsername] = createSignal("");
  const [sftpAuthType, setSftpAuthType] = createSignal<"password" | "key">("password");
  const [sftpPassword, setSftpPassword] = createSignal("");
  const [sftpKeyPath, setSftpKeyPath] = createSignal("");
  const [sftpRemotePath, setSftpRemotePath] = createSignal("/");

  // WebDAV fields
  const [webdavUrl, setWebdavUrl] = createSignal("");
  const [webdavUsername, setWebdavUsername] = createSignal("");
  const [webdavPassword, setWebdavPassword] = createSignal("");
  const [webdavRootPath, setWebdavRootPath] = createSignal("/ShadowVault");

  // OAuth fields
  const [oauthProvider, setOauthProvider] = createSignal<"onedrive" | "gdrive" | "dropbox">("onedrive");
  const [oauthFolderPath, setOauthFolderPath] = createSignal("/ShadowVault");
  const [oauthConfig, setOauthConfig] = createSignal<OAuthConfig | null>(null);
  const [oauthStatus, setOauthStatus] = createSignal<"idle" | "waiting" | "done" | "error">("idle");
  const [oauthError, setOauthError] = createSignal("");
  const [testing, setTesting] = createSignal(false);

  // Common fields
  const [schedule, setSchedule] = createSignal<ScheduleType>({ type: "Interval", value: { minutes: 60 } });
  const [maxVersions, setMaxVersions] = createSignal(10);
  const [naming, setNaming] = createSignal<"Timestamp" | "Index" | "Overwrite">("Timestamp");
  const [saving, setSaving] = createSignal(false);
  const [showUpgrade, setShowUpgrade] = createSignal(false);
  const isLicensed = () => store.licenseStatus === "valid";
  const LOW_SPACE_THRESHOLD = 500 * 1024 * 1024;

  const retention = (): RetentionPolicy => ({ max_versions: maxVersions(), naming: naming() });
  const r2Endpoint = () =>
    accountId().trim() ? `https://${accountId().trim()}.r2.cloudflarestorage.com` : undefined;

  const reset = () => {
    setStep(1); setName(""); setSourcePath(""); setSourceType("Directory");
    setDestType("Local"); setDestPath(""); setAvailBytes(null);
    setCloudProvider("S3"); setBucket(""); setRegion("us-east-1");
    setAccessKeyId(""); setSecretAccessKey(""); setAccountId(""); setPrefix("");
    setSftpHost(""); setSftpPort(22); setSftpUsername(""); setSftpAuthType("password");
    setSftpPassword(""); setSftpKeyPath(""); setSftpRemotePath("/");
    setWebdavUrl(""); setWebdavUsername(""); setWebdavPassword(""); setWebdavRootPath("/ShadowVault");
    setOauthProvider("onedrive"); setOauthFolderPath("/ShadowVault");
    setOauthConfig(null); setOauthStatus("idle"); setOauthError("");
    setSchedule({ type: "Interval", value: { minutes: 60 } });
    setMaxVersions(10); setNaming("Timestamp"); setSaving(false);
  };

  createEffect(() => {
    if (props.open && props.prefillPath) {
      setSourcePath(props.prefillPath);
      setSourceType(props.prefillType ?? "Directory");
    }
  });

  const handleClose = () => { reset(); props.onClose(); };

  const pickSource = async () => {
    try {
      const picked = sourceType() === "Directory"
        ? await api.fs.pickDirectory()
        : await api.fs.pickFile();
      if (picked) setSourcePath(picked);
    } catch { toast.error(t("add_src_pick_err")); }
  };

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
    if (destType() === "WebDav") {
      if (!webdavUrl().trim() || !webdavUsername().trim()) {
        toast.error(t("webdav_fields_required")); return;
      }
      setTesting(true);
      try {
        await api.cloud.testWebDavConnection(
          webdavUrl().trim(), webdavUsername().trim(),
          webdavPassword().trim(), webdavRootPath().trim() || "/ShadowVault",
        );
        toast.success(t("cloud_connection_ok"));
      } catch (e: any) {
        toast.error(e?.message ?? t("cloud_connection_err"));
      } finally { setTesting(false); }
      return;
    }
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
      toast.error(t("cloud_fields_required")); return;
    }
    if (prov === "R2" && !accountId().trim()) {
      toast.error(t("cloud_r2_account_required")); return;
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

  const destDisplayPath = () => {
    const dt = destType();
    if (dt === "Local") return destPath();
    if (dt === "Sftp") return `sftp://${sftpHost().trim()}${sftpRemotePath().trim() || "/"}`;
    if (dt === "WebDav") return `webdav://${webdavUrl().trim().replace(/^https?:\/\//, "")}${webdavRootPath().trim() || "/ShadowVault"}`;
    if (dt === "OneDrive" || dt === "GoogleDrive" || dt === "Dropbox") return `${oauthProvider()}://${oauthFolderPath().trim() || "/ShadowVault"}`;
    const prov = cloudProvider() as "S3" | "R2";
    return `${prov.toLowerCase()}://${bucket().trim()}/${prefix().trim()}`;
  };

  const validateStep2 = () => {
    const dt = destType();
    if (dt === "Local") {
      if (!destPath().trim()) { toast.error(t("add_src_dest_req")); return false; }
    } else if (dt === "Sftp") {
      if (!sftpHost().trim() || !sftpUsername().trim()) { toast.error(t("sftp_fields_required")); return false; }
    } else if (dt === "WebDav") {
      if (!webdavUrl().trim() || !webdavUsername().trim()) { toast.error(t("webdav_fields_required")); return false; }
    } else if (dt === "OneDrive" || dt === "GoogleDrive" || dt === "Dropbox") {
      if (!oauthConfig()) { toast.error(t("oauth_not_connected")); return false; }
    } else {
      if (!bucket().trim() || !accessKeyId().trim() || !secretAccessKey().trim()) { toast.error(t("cloud_fields_required")); return false; }
    }
    return true;
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const source = await api.sources.create(name(), sourcePath(), sourceType());
      const dt = destType();

      if (dt === "Local") {
        await api.destinations.add(source.id, destPath(), schedule(), retention(), [], false, "Local", null, null, null);
      } else if (dt === "Sftp") {
        const sftpConfig: SftpConfig = {
          host: sftpHost().trim(),
          port: sftpPort(),
          username: sftpUsername().trim(),
          auth_type: sftpAuthType(),
          password: sftpAuthType() === "password" ? sftpPassword().trim() : undefined,
          private_key: sftpAuthType() === "key" ? sftpKeyPath().trim() : undefined,
          remote_path: sftpRemotePath().trim() || "/",
        };
        await api.destinations.add(source.id, destDisplayPath(), schedule(), retention(), [], false, "Sftp", null, sftpConfig, null);
      } else if (dt === "WebDav") {
        const webdavConfig: WebDavConfig = {
          url: webdavUrl().trim(),
          username: webdavUsername().trim(),
          password: webdavPassword().trim(),
          root_path: webdavRootPath().trim() || "/ShadowVault",
        };
        await api.destinations.add(source.id, destDisplayPath(), schedule(), retention(), [], false, "WebDav", null, null, null, false, null, webdavConfig);
      } else if (dt === "OneDrive" || dt === "GoogleDrive" || dt === "Dropbox") {
        await api.destinations.add(source.id, destDisplayPath(), schedule(), retention(), [], false, dt, null, null, oauthConfig());
      } else {
        const prov = cloudProvider() as "S3" | "R2";
        const cloudConfig: S3Config = {
          provider: prov,
          bucket: bucket().trim(),
          region: prov === "R2" ? "auto" : region().trim(),
          access_key_id: accessKeyId().trim(),
          secret_access_key: secretAccessKey().trim(),
          endpoint_url: prov === "R2" ? r2Endpoint() : undefined,
          prefix: prefix().trim(),
        };
        await api.destinations.add(source.id, destDisplayPath(), schedule(), retention(), [], false, prov as DestinationType, cloudConfig, null);
      }

      props.onCreated();
      handleClose();
    } catch (e: any) {
      toast.error(e?.message ?? t("add_src_save_err"));
    } finally { setSaving(false); }
  };

  const scheduleDescription = () => {
    const s = schedule();
    if (s.type === "Interval") return `${t("schedule_interval").replace("X", String(s.value.minutes))}`;
    if (s.type === "Cron") return `Cron: ${s.value.expression}`;
    if (s.type === "OnChange") return t("schedule_onchange");
    return t("schedule_manual");
  };

  const stepTitles = () => [t("add_src_step1"), t("add_src_step2"), t("add_src_step3")];

  return (
    <>
    <Modal
      open={props.open}
      closeOnBackdrop={false}
      onClose={handleClose}
      title={`Step ${step()}/3: ${stepTitles()[step() - 1]}`}
      footer={
        <div class={styles.footerRow}>
          <Button variant="ghost" onClick={handleClose}>{t("btn_cancel")}</Button>
          {step() > 1 && <Button variant="ghost" onClick={() => setStep(s => s - 1)}>{t("btn_back")}</Button>}
          {step() < 3 && (
            <Button onClick={() => {
              if (step() === 1) {
                if (!name().trim()) { toast.error(t("add_src_name_req")); return; }
                if (!sourcePath().trim()) { toast.error(t("add_src_path_req")); return; }
              }
              if (step() === 2 && !validateStep2()) return;
              setStep(s => s + 1);
            }}>{t("btn_next")}</Button>
          )}
          {step() === 3 && (
            <Button onClick={handleSave} disabled={saving()}>
              {saving() ? t("btn_saving") : t("btn_save")}
            </Button>
          )}
        </div>
      }
    >
      {/* Step 1 */}
      <Show when={step() === 1}>
        <div class={styles.field}>
          <label class={styles.label}>{t("add_src_name_label")}</label>
          <input class={styles.input} type="text" placeholder={t("add_src_name_ph")}
            value={name()} onInput={(e) => setName(e.currentTarget.value)} />
        </div>
        <div class={styles.field}>
          <label class={styles.label}>{t("add_src_type_label")}</label>
          <div class={styles.radioGroup}>
            {(["Directory", "File"] as const).map((tp) => (
              <label class={styles.radioLabel}>
                <input type="radio" checked={sourceType() === tp} onChange={() => setSourceType(tp)} />
                {tp === "Directory" ? t("add_src_folder") : t("add_src_file")}
              </label>
            ))}
          </div>
        </div>
        <div class={styles.field}>
          <label class={styles.label}>{t("add_src_path_label")}</label>
          <div class={styles.inputRow}>
            <input class={`${styles.input} ${styles.inputFlex}`} type="text"
              placeholder={sourceType() === "Directory" ? "/home/user/belgeler" : "/home/user/dosya.txt"}
              value={sourcePath()} onInput={(e) => setSourcePath(e.currentTarget.value)} />
            <Button variant="ghost" size="sm" onClick={pickSource}>{t("btn_browse")}</Button>
          </div>
        </div>
      </Show>

      {/* Step 2 */}
      <Show when={step() === 2}>
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
            data-active={String(destType() === "OneDrive" || destType() === "GoogleDrive" || destType() === "Dropbox")}
            onClick={() => { setDestType("OneDrive"); setOauthProvider("onedrive"); setCloudProvider("OAuth"); }}
          >
            {t("dest_type_oauth")}
          </button>
          <button
            class={styles.typeTab}
            data-active={String(destType() === "WebDav")}
            onClick={() => setDestType("WebDav")}
          >
            {t("dest_type_webdav")}
          </button>
        </div>

        {/* Local */}
        <Show when={destType() === "Local"}>
          <div class={styles.field}>
            <label class={styles.label}>{t("add_dest_folder")}</label>
            <div class={styles.inputRow}>
              <input class={`${styles.input} ${styles.inputFlex}`} type="text" placeholder="/backup/hedef"
                value={destPath()} onInput={(e) => { setDestPath(e.currentTarget.value); checkDiskSpace(e.currentTarget.value); }} />
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

        {/* Cloud (S3/R2) */}
        <Show when={destType() === "S3" || destType() === "R2"}>
          <div class={styles.field}>
            <label class={styles.label}>{t("cloud_provider")}</label>
            <select class={styles.input} value={cloudProvider()}
              onChange={(e) => {
                const v = e.currentTarget.value as "S3" | "R2";
                setCloudProvider(v); setDestType(v);
                if (v === "S3") setRegion("us-east-1");
              }}>
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

        {/* SFTP */}
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

        {/* WebDAV */}
        <Show when={destType() === "WebDav"}>
          <div class={styles.field}>
            <label class={styles.label}>{t("webdav_url")}</label>
            <input class={styles.input} type="text" placeholder={t("webdav_url_ph")}
              value={webdavUrl()} onInput={e => setWebdavUrl(e.currentTarget.value)} />
            <div class={styles.hint}>{t("webdav_url_hint")}</div>
          </div>
          <div class={styles.retentionRow}>
            <div class={styles.retentionCol} style={{ flex: "2" }}>
              <label class={styles.label}>{t("webdav_username")}</label>
              <input class={styles.input} type="text" placeholder={t("webdav_username_ph")}
                value={webdavUsername()} onInput={e => setWebdavUsername(e.currentTarget.value)} />
            </div>
            <div class={styles.retentionCol} style={{ flex: "2" }}>
              <label class={styles.label}>{t("webdav_password")}</label>
              <input class={styles.input} type="password" placeholder="••••••••"
                value={webdavPassword()} onInput={e => setWebdavPassword(e.currentTarget.value)} />
            </div>
          </div>
          <div class={styles.field}>
            <label class={styles.label}>{t("webdav_root_path")}</label>
            <input class={styles.input} type="text" placeholder="/ShadowVault"
              value={webdavRootPath()} onInput={e => setWebdavRootPath(e.currentTarget.value)} />
          </div>
          <div class={styles.field}>
            <Button variant="ghost" size="sm" onClick={handleTestConnection} disabled={testing()}>
              {testing() ? t("cloud_testing") : t("cloud_test_btn")}
            </Button>
          </div>
        </Show>

        {/* OAuth (OneDrive / Google Drive / Dropbox) */}
        <Show when={destType() === "OneDrive" || destType() === "GoogleDrive" || destType() === "Dropbox"}>
          <div class={styles.field}>
            <label class={styles.label}>{t("oauth_provider")}</label>
            <select class={styles.input} value={oauthProvider()}
              onChange={e => {
                const v = e.currentTarget.value as "onedrive" | "gdrive" | "dropbox";
                setOauthProvider(v);
                setDestType(v === "onedrive" ? "OneDrive" : v === "gdrive" ? "GoogleDrive" : "Dropbox");
                setOauthConfig(null); setOauthStatus("idle");
              }}>
              <option value="onedrive">Microsoft OneDrive</option>
              <option value="gdrive">Google Drive</option>
              <option value="dropbox">Dropbox</option>
            </select>
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
                  setOauthStatus("waiting"); setOauthError("");
                  try {
                    const cfg = await api.oauth.runFlow(oauthProvider(), oauthFolderPath().trim() || "/ShadowVault");
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
      </Show>

      {/* Step 3 */}
      <Show when={step() === 3}>
        <div class={styles.summaryCards}>
          <div class={styles.summaryCard}>
            <div class={styles.summarySection}>{t("sum_source")}</div>
            <div class={styles.summaryRows}>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_name")}</span>
                <span class={styles.summaryVal}>{name()}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_type")}</span>
                <span class={styles.summaryVal}>{sourceType() === "Directory" ? t("add_src_folder") : t("add_src_file")}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_path")}</span>
                <span class={styles.summaryVal}>{sourcePath()}</span>
              </div>
            </div>
          </div>
          <div class={styles.summaryCard}>
            <div class={styles.summarySection}>{t("sum_target")}</div>
            <div class={styles.summaryRows}>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_path")}</span>
                <span class={styles.summaryVal}>{destDisplayPath()}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_schedule")}</span>
                <span class={styles.summaryVal}>{scheduleDescription()}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_max_ver")}</span>
                <span class={styles.summaryVal}>{maxVersions()}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_naming")}</span>
                <span class={styles.summaryVal}>{naming()}</span>
              </div>
            </div>
          </div>
        </div>
      </Show>
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

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}
