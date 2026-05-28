import { useMemo, useState } from "react";
import { api } from "../api/client";
import { categories } from "../i18n";
import { useLang } from "../hooks/useLang";
import { usePhotos } from "../hooks/usePhotos";
import type { Photo } from "../types";
import { ArrowIcon, CloseIcon, DownloadIcon, ExternalLinkIcon, EyeOffIcon, LockIcon } from "./Icons";

export function PublicGallery() {
  const { lang, setLang, t } = useLang();
  const { photos, loading, error } = usePhotos(false);
  const [category, setCategory] = useState("all");
  const [activeId, setActiveId] = useState<number | null>(null);
  const [unlocked, setUnlocked] = useState<Set<number>>(() => new Set());

  const filtered = useMemo(() => (
    category === "all" ? photos : photos.filter((photo) => photo.cat === category)
  ), [category, photos]);
  const active = filtered.find((photo) => photo.id === activeId) ?? null;
  const activeIndex = filtered.findIndex((photo) => photo.id === activeId);

  const move = (direction: -1 | 1) => {
    if (!filtered.length) return;
    const next = activeIndex < 0
      ? 0
      : (activeIndex + direction + filtered.length) % filtered.length;
    setActiveId(filtered[next].id);
  };

  return (
    <div className="site-shell">
      <header className="topbar">
        <a className="brand" href="/">
          <span>{t.siteName}</span>
          <small>{t.siteSub}</small>
        </a>
        <nav className="category-nav" aria-label="categories">
          {categories.map((item) => (
            <button key={item.key} className={category === item.key ? "active" : ""} onClick={() => setCategory(item.key)}>
              {item[lang]}
            </button>
          ))}
        </nav>
        <div className="top-actions">
          <div className="lang-switch">
            <button className={lang === "zh" ? "active" : ""} onClick={() => setLang("zh")}>中</button>
            <button className={lang === "en" ? "active" : ""} onClick={() => setLang("en")}>EN</button>
          </div>
          {/* admin 入口已隐藏；通过 URL /admin 直接访问 */}
        </div>
      </header>

      <main>
        <section className="intro">
          <div>
            <p>{t.issue}</p>
            <h1>{t.headline}</h1>
          </div>
          <aside>
            <p>{t.intro}</p>
            <div className="intro-meta">
              <span>{t.frames(photos.length)}</span>
              <span>{t.updated}</span>
            </div>
          </aside>
        </section>

        {loading && <div className="state-line">Loading...</div>}
        {error && <div className="state-line error">{error}</div>}
        {/* key={category} 让切类别时整个 masonry remount，
            触发 .photo-card 的 cardIn 动画 + 重新跑图片 onLoad 淡入 */}
        <section className="masonry" key={category}>
          {filtered.map((photo) => (
            <PhotoCard key={photo.id} photo={photo} unlocked={unlocked.has(photo.id)} onOpen={() => setActiveId(photo.id)} lang={lang} />
          ))}
        </section>
      </main>

      <footer className="footer">
        <div>
          <strong>{t.siteName}</strong>
          <span>{t.range}</span>
        </div>
      </footer>

      <Lightbox
        photo={active}
        lang={lang}
        unlocked={active ? unlocked.has(active.id) : false}
        onClose={() => setActiveId(null)}
        onPrev={() => move(-1)}
        onNext={() => move(1)}
        onUnlocked={(id) => setUnlocked((prev) => new Set(prev).add(id))}
      />
    </div>
  );
}

function PhotoCard({ photo, unlocked, onOpen, lang }: { photo: Photo; unlocked: boolean; onOpen: () => void; lang: "zh" | "en" }) {
  const hidden = photo.privacy === "private" || (photo.privacy === "locked" && !unlocked);
  // 优先 webp（现代浏览器自动选）→ medium JPG → 兜底 src
  const jpgSrc = photo.variants?.medium ?? photo.src;
  const webpSrc = photo.variants?.webp;
  return (
    <figure className="photo-card" onClick={onOpen}>
      <div className="photo-media">
        <picture>
          {webpSrc && <source srcSet={webpSrc} type="image/webp" />}
          <img
            src={jpgSrc}
            alt={photo.title[lang]}
            className={hidden ? "obscured" : ""}
            loading="lazy"
            decoding="async"
            // data-loaded 触发淡入；缓存命中也会 fire onLoad，所以无 flicker
            onLoad={(event) => { event.currentTarget.dataset.loaded = "true"; }}
          />
        </picture>
        {hidden && (
          <div className="photo-shield">
            {photo.privacy === "private" ? <EyeOffIcon /> : <LockIcon />}
            <span>{photo.privacy}</span>
          </div>
        )}
      </div>
      <figcaption>
        <div>
          <strong>{photo.title[lang]}</strong>
          <span>{photo.loc[lang]} · {photo.date}</span>
        </div>
      </figcaption>
    </figure>
  );
}

function Lightbox({
  photo,
  lang,
  unlocked,
  onClose,
  onPrev,
  onNext,
  onUnlocked
}: {
  photo: Photo | null;
  lang: "zh" | "en";
  unlocked: boolean;
  onClose: () => void;
  onPrev: () => void;
  onNext: () => void;
  onUnlocked: (id: number) => void;
}) {
  const { t } = useLang();
  const [passcode, setPasscode] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [downloading, setDownloading] = useState(false);
  if (!photo) return null;

  const locked = photo.privacy === "locked" && !unlocked;
  const privatePhoto = photo.privacy === "private";
  const canAccessOriginal = !privatePhoto && !locked;
  const originalUrl = photo.variants?.original;

  const unlock = async () => {
    const result = await api.unlockPhoto(photo.id, passcode);
    if (result.unlocked) {
      setError(null);
      onUnlocked(photo.id);
    } else {
      setError("Invalid passcode");
    }
  };

  // 「保存原图」：fetch + blob + 触发下载（兼容跨域 + Content-Disposition 缺失场景）
  const download = async (event: React.MouseEvent) => {
    event.stopPropagation();
    if (!originalUrl || downloading) return;
    setDownloading(true);
    try {
      const resp = await fetch(originalUrl);
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const blob = await resp.blob();
      const a = document.createElement("a");
      a.href = URL.createObjectURL(blob);
      // 文件名取 slug + 从 URL path 末段提取的扩展名
      const ext = (originalUrl.match(/\.([a-zA-Z0-9]{2,5})(?:\?|$)/)?.[1] ?? "jpg").toLowerCase();
      a.download = `${photo.slug || photo.id}.${ext}`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(a.href);
    } catch (err) {
      console.error("download failed", err);
    } finally {
      setDownloading(false);
    }
  };

  const viewOriginal = (event: React.MouseEvent) => {
    event.stopPropagation();
    if (!originalUrl) return;
    window.open(originalUrl, "_blank", "noopener,noreferrer");
  };

  return (
    <div className="lightbox" onClick={onClose}>
      <button className="icon-button close" onClick={onClose} aria-label={t.close as string}><CloseIcon /></button>
      {canAccessOriginal && originalUrl && (
        <div className="lightbox-actions" onClick={(event) => event.stopPropagation()}>
          <button
            className="icon-button"
            onClick={viewOriginal}
            aria-label={t.viewOriginal as string}
            title={t.viewOriginal as string}
          >
            <ExternalLinkIcon size={18} />
          </button>
          <button
            className="icon-button"
            onClick={(event) => void download(event)}
            aria-label={t.download as string}
            title={t.download as string}
            disabled={downloading}
          >
            <DownloadIcon size={18} />
          </button>
        </div>
      )}
      <button className="icon-button prev" onClick={(event) => { event.stopPropagation(); onPrev(); }} aria-label={t.prev as string}><ArrowIcon direction="left" /></button>
      <button className="icon-button next" onClick={(event) => { event.stopPropagation(); onNext(); }} aria-label={t.next as string}><ArrowIcon /></button>
      <div className="lightbox-inner" onClick={(event) => event.stopPropagation()}>
        {privatePhoto ? (
          <div className="locked-panel"><EyeOffIcon size={28} /><p>{t.privateOnly as string}</p></div>
        ) : locked ? (
          <div className="locked-panel">
            <LockIcon size={28} />
            <h2>{photo.title[lang]}</h2>
            <p>{t.lockedHint as string}</p>
            <input value={passcode} onChange={(event) => setPasscode(event.target.value)} onKeyDown={(event) => event.key === "Enter" && void unlock()} placeholder={t.passcode as string} type="password" />
            <button className="primary" onClick={() => void unlock()}>{t.unlock as string}</button>
            {error && <span className="form-error">{error}</span>}
          </div>
        ) : (
          <img
            // key 让切 prev/next 时 React remount img，重放 CSS 入场动画
            key={photo.id}
            className="lightbox-image"
            src={photo.variants?.full ?? photo.src.replace(/w=\d+/, "w=1800")}
            alt={photo.title[lang]}
            decoding="async"
            fetchPriority="high"
          />
        )}
        <div className="lightbox-caption">
          <div>
            <strong>{photo.title[lang]}</strong>
            <span>{photo.loc[lang]} · {photo.date}</span>
          </div>
          <span>{categories.find((item) => item.key === photo.cat)?.[lang]}</span>
        </div>
      </div>
    </div>
  );
}
