import {
  TbOutlineFile,
  TbOutlineFileText,
  TbOutlinePhoto,
  TbOutlineVideo,
  TbOutlineMusic,
  TbOutlineFileZip,
  TbOutlineFileCode,
  TbOutlineFolder,
  TbOutlineFolderOpen,
} from "solid-icons/tb";

const IMAGE_EXTS = new Set(["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg", "ico", "tiff"]);
const VIDEO_EXTS = new Set(["mp4", "mov", "avi", "mkv", "webm", "flv", "wmv"]);
const AUDIO_EXTS = new Set(["mp3", "wav", "flac", "aac", "ogg", "m4a"]);
const TEXT_EXTS = new Set(["txt", "md", "csv", "log", "rtf", "doc", "docx", "xls", "xlsx", "ppt", "pptx"]);
const CODE_EXTS = new Set(["js", "ts", "jsx", "tsx", "py", "rs", "go", "java", "c", "cpp", "h", "html", "css", "json", "yaml", "yml", "toml", "sh", "bash", "zsh"]);
const ZIP_EXTS = new Set(["zip", "tar", "gz", "bz2", "xz", "7z", "rar"]);

export function FileIcon(props: { name: string; isDir?: boolean; open?: boolean; size?: number }) {
  const size = () => props.size ?? 18;

  if (props.isDir) {
    return props.open
      ? <TbOutlineFolderOpen size={size()} color="var(--yellow)" />
      : <TbOutlineFolder size={size()} color="var(--yellow)" />;
  }

  const ext = props.name.split(".").pop()?.toLowerCase() ?? "";

  if (IMAGE_EXTS.has(ext)) return <TbOutlinePhoto size={size()} color="#58b8e8" />;
  if (VIDEO_EXTS.has(ext)) return <TbOutlineVideo size={size()} color="#9b7df5" />;
  if (AUDIO_EXTS.has(ext)) return <TbOutlineMusic size={size()} color="#f08a5d" />;
  if (ext === "pdf") return <TbOutlineFileText size={size()} color="#e55c5c" />;
  if (ZIP_EXTS.has(ext)) return <TbOutlineFileZip size={size()} color="#e5c55c" />;
  if (CODE_EXTS.has(ext)) return <TbOutlineFileCode size={size()} color="#5ce5c5" />;
  if (TEXT_EXTS.has(ext)) return <TbOutlineFileText size={size()} color="var(--text-secondary)" />;

  return <TbOutlineFile size={size()} color="var(--text-secondary)" />;
}

export function isImageFile(name: string): boolean {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  return IMAGE_EXTS.has(ext);
}
