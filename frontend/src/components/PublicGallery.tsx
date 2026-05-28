import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
  type WheelEvent as ReactWheelEvent
} from "react";
import { api } from "../api/client";
import { categories } from "../i18n";
import { useLang } from "../hooks/useLang";
import { usePhotos } from "../hooks/usePhotos";
import type { Photo } from "../types";
import { ArrowIcon, CloseIcon, DownloadIcon, EyeOffIcon, FitIcon, LockIcon, ZoomInIcon, ZoomOutIcon } from "./Icons";

export function PublicGallery() {
  const { lang, setLang, t } = useLang();
  const { photos, loading, error } = usePhotos(false);
  const [category, setCategory] = useState("all");
  const [activeId, setActiveId] = useState<number | null>(null);
  const [unlocked, setUnlocked] = useState<Set<number>>(() => new Set());
  const masonryRef = useRef<HTMLElement | null>(null);

  const filtered = useMemo(() => (
    category === "all" ? photos : photos.filter((photo) => photo.cat === category)
  ), [category, photos]);
  const introLines = t.introLines;
  const active = filtered.find((photo) => photo.id === activeId) ?? null;
  const activeIndex = filtered.findIndex((photo) => photo.id === activeId);

  const move = (direction: -1 | 1) => {
    if (!filtered.length) return;
    const next = activeIndex < 0
      ? 0
      : (activeIndex + direction + filtered.length) % filtered.length;
    setActiveId(filtered[next].id);
  };

  useEffect(() => {
    const root = masonryRef.current;
    if (!root) return;

    const cards = Array.from(root.querySelectorAll<HTMLElement>(".photo-card"));
    if (!cards.length) return;
    const images = cards
      .map((card) => card.querySelector("img"))
      .filter((image): image is HTMLImageElement => Boolean(image));

    cards.forEach((card) => {
      card.classList.remove("is-visible");
      card.style.setProperty("--reveal-delay", "0ms");
    });

    let layoutFrame = 0;
    let revealFrame = 0;
    let fallbackTimer = 0;
    let didReveal = false;

    const revealTopDown = () => {
      if (didReveal) return;
      didReveal = true;
      window.clearTimeout(fallbackTimer);

      layoutFrame = window.requestAnimationFrame(() => {
        const topValues = cards.map((card) => card.offsetTop);
        const minTop = Math.min(...topValues);

        cards.forEach((card, index) => {
          const delay = Math.min(760, Math.max(0, (topValues[index] - minTop) * 0.42));
          card.style.setProperty("--reveal-delay", `${Math.round(delay)}ms`);
        });

        revealFrame = window.requestAnimationFrame(() => {
          cards.forEach((card) => card.classList.add("is-visible"));
        });
      });
    };

    const revealAfterMeasurableLayout = () => {
      if (cards.some((card) => card.getBoundingClientRect().height > 80)) {
        revealTopDown();
      }
    };

    images.forEach((image) => {
      if (image.complete) return;
      image.addEventListener("load", revealAfterMeasurableLayout, { once: true });
      image.addEventListener("error", revealAfterMeasurableLayout, { once: true });
    });

    layoutFrame = window.requestAnimationFrame(revealAfterMeasurableLayout);
    fallbackTimer = window.setTimeout(revealTopDown, 520);

    return () => {
      images.forEach((image) => {
        image.removeEventListener("load", revealAfterMeasurableLayout);
        image.removeEventListener("error", revealAfterMeasurableLayout);
      });
      window.clearTimeout(fallbackTimer);
      window.cancelAnimationFrame(layoutFrame);
      window.cancelAnimationFrame(revealFrame);
    };
  }, [category, filtered]);

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
          <div className="intro-copy">
            <p className="intro-kicker">{t.issue}</p>
            <h1 className="intro-title" aria-label={t.headline}>
              <span className="intro-title-lead" aria-hidden="true">{t.headlineLead}</span>
              <span className="intro-title-rest" aria-hidden="true">{t.headlineRest}</span>
            </h1>
          </div>
          <aside>
            <TypewriterText lines={introLines} />
            <div className="intro-meta">
              <span>{t.frames(photos.length)}</span>
              <span>{t.updated}</span>
            </div>
          </aside>
        </section>

        {loading && <div className="state-line">Loading...</div>}
        {error && <div className="state-line error">{error}</div>}
        {/* key={category} 让切类别时整个 masonry remount，重新跑图片 onLoad 淡入。 */}
        <section className="masonry" key={category} ref={masonryRef}>
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

function TypewriterText({ lines }: { lines: string[] }) {
  const [lineIndex, setLineIndex] = useState(0);
  const [charCount, setCharCount] = useState(0);
  const [deleting, setDeleting] = useState(false);
  const lineSignature = lines.join("\n");
  const currentLine = lines[lineIndex] ?? "";
  const currentChars = Array.from(currentLine);

  useEffect(() => {
    setLineIndex(0);
    setCharCount(0);
    setDeleting(false);
  }, [lineSignature]);

  useEffect(() => {
    if (!lines.length) return;

    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setLineIndex(0);
      setCharCount(Array.from(lines[0]).length);
      setDeleting(false);
      return;
    }

    const atEnd = charCount >= currentChars.length;
    const atStart = charCount <= 0;
    const delay = deleting
      ? 34
      : atEnd
        ? 1700
        : atStart && lineIndex > 0
          ? 260
          : 68;

    const timer = window.setTimeout(() => {
      if (!deleting && atEnd) {
        setDeleting(true);
        return;
      }

      if (deleting && atStart) {
        setDeleting(false);
        setLineIndex((index) => (index + 1) % lines.length);
        return;
      }

      setCharCount((count) => count + (deleting ? -1 : 1));
    }, delay);

    return () => window.clearTimeout(timer);
  }, [charCount, currentChars.length, deleting, lineIndex, lines]);

  return (
    <p className="typewriter-copy" aria-label={currentLine}>
      <span aria-hidden="true">{currentChars.slice(0, charCount).join("")}</span>
      <span className="typewriter-caret" aria-hidden="true" />
    </p>
  );
}

function PhotoCard({ photo, unlocked, onOpen, lang }: { photo: Photo; unlocked: boolean; onOpen: () => void; lang: "zh" | "en" }) {
  const hidden = photo.privacy === "private" || (photo.privacy === "locked" && !unlocked);
  // 优先 webp（现代浏览器自动选）→ medium JPG → 兜底 src
  const jpgSrc = photo.variants?.medium ?? photo.src;
  const webpSrc = photo.variants?.webp;
  return (
    <figure
      className="photo-card"
      onClick={onOpen}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onOpen();
        }
      }}
      role="button"
      tabIndex={0}
    >
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

type ImageQuality = "full" | "original";
type ViewerOffset = { x: number; y: number };

const minZoom = 1;
const maxZoom = 4;

function clampZoom(value: number) {
  return Math.min(maxZoom, Math.max(minZoom, Math.round(value * 100) / 100));
}

function highResUrl(photo: Photo) {
  return photo.variants?.full ?? photo.src.replace(/w=\d+/, "w=1800");
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
  const [imageQuality, setImageQuality] = useState<ImageQuality>("full");
  const [scale, setScale] = useState(1);
  const [offset, setOffset] = useState<ViewerOffset>({ x: 0, y: 0 });
  const [dragging, setDragging] = useState(false);
  const dragRef = useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    startOffset: ViewerOffset;
  } | null>(null);

  useEffect(() => {
    setImageQuality("full");
    setScale(1);
    setOffset({ x: 0, y: 0 });
    setDragging(false);
    setPasscode("");
    setError(null);
    dragRef.current = null;
  }, [photo?.id]);

  useEffect(() => {
    if (!photo) return;

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      if (event.key === "Escape") {
        onClose();
      } else if (event.key === "ArrowLeft") {
        onPrev();
      } else if (event.key === "ArrowRight") {
        onNext();
      } else if (event.key === "+" || event.key === "=") {
        setScale((value) => clampZoom(value + 0.25));
      } else if (event.key === "-") {
        setScale((value) => {
          const next = clampZoom(value - 0.25);
          if (next === minZoom) setOffset({ x: 0, y: 0 });
          return next;
        });
      } else if (event.key === "0") {
        setScale(1);
        setOffset({ x: 0, y: 0 });
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose, onNext, onPrev, photo]);

  if (!photo) return null;

  const locked = photo.privacy === "locked" && !unlocked;
  const privatePhoto = photo.privacy === "private";
  const canAccessOriginal = !privatePhoto && !locked;
  const fullUrl = highResUrl(photo);
  const originalUrl = photo.variants?.original;
  const hasOriginal = canAccessOriginal && Boolean(originalUrl);
  const showingOriginal = imageQuality === "original" && hasOriginal;
  const imageUrl = showingOriginal && originalUrl ? originalUrl : fullUrl;

  const resetView = () => {
    setScale(1);
    setOffset({ x: 0, y: 0 });
    setDragging(false);
    dragRef.current = null;
  };

  const zoomBy = (delta: number) => {
    setScale((value) => {
      const next = clampZoom(value + delta);
      if (next === minZoom) setOffset({ x: 0, y: 0 });
      return next;
    });
  };

  const switchQuality = () => {
    if (!hasOriginal) return;
    setImageQuality((value) => value === "original" ? "full" : "original");
    resetView();
  };

  const onStageWheel = (event: ReactWheelEvent<HTMLDivElement>) => {
    event.stopPropagation();
    event.preventDefault();
    zoomBy(event.deltaY > 0 ? -0.2 : 0.2);
  };

  const onStageDoubleClick = (event: React.MouseEvent<HTMLDivElement>) => {
    event.stopPropagation();
    if (scale > minZoom) {
      resetView();
    } else {
      setScale(2.25);
    }
  };

  const onPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (scale <= minZoom) return;
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      startOffset: offset,
    };
    setDragging(true);
  };

  const onPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    event.stopPropagation();
    setOffset({
      x: drag.startOffset.x + event.clientX - drag.startX,
      y: drag.startOffset.y + event.clientY - drag.startY,
    });
  };

  const onPointerEnd = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (dragRef.current?.pointerId === event.pointerId) {
      event.stopPropagation();
      dragRef.current = null;
      setDragging(false);
    }
  };

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

  return (
    <div className="lightbox" onClick={onClose}>
      <button className="icon-button close" onClick={onClose} aria-label={t.close as string}><CloseIcon /></button>
      {!privatePhoto && !locked && (
        <div className="lightbox-actions" onClick={(event) => event.stopPropagation()}>
          <button
            className="icon-button"
            onClick={() => zoomBy(-0.25)}
            aria-label={t.zoomOut as string}
            title={t.zoomOut as string}
            disabled={scale <= minZoom}
          >
            <ZoomOutIcon size={18} />
          </button>
          <button
            className="icon-button"
            onClick={resetView}
            aria-label={t.fitToScreen as string}
            title={t.fitToScreen as string}
          >
            <FitIcon size={18} />
          </button>
          <button
            className="icon-button"
            onClick={() => zoomBy(0.25)}
            aria-label={t.zoomIn as string}
            title={t.zoomIn as string}
            disabled={scale >= maxZoom}
          >
            <ZoomInIcon size={18} />
          </button>
          <button
            className={`viewer-quality${showingOriginal ? " active" : ""}`}
            onClick={switchQuality}
            aria-label={showingOriginal ? t.viewFull as string : t.viewOriginal as string}
            title={showingOriginal ? t.viewFull as string : t.viewOriginal as string}
            disabled={!hasOriginal}
          >
            {showingOriginal ? (lang === "zh" ? "高清" : "Full") : (lang === "zh" ? "原图" : "Original")}
          </button>
          {hasOriginal && (
            <button
              className="icon-button"
              onClick={(event) => void download(event)}
              aria-label={t.download as string}
              title={t.download as string}
              disabled={downloading}
            >
              <DownloadIcon size={18} />
            </button>
          )}
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
          <div
            className={`lightbox-stage${scale > minZoom ? " zoomed" : ""}${dragging ? " dragging" : ""}`}
            onWheel={onStageWheel}
            onDoubleClick={onStageDoubleClick}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerEnd}
            onPointerCancel={onPointerEnd}
          >
            <img
              // key 让切 prev/next 或切清晰度时 React remount img，重放 CSS 入场动画
              key={`${photo.id}-${imageQuality}`}
              className="lightbox-image"
              src={imageUrl}
              alt={photo.title[lang]}
              decoding="async"
              fetchPriority="high"
              draggable={false}
              style={{ transform: `translate3d(${offset.x}px, ${offset.y}px, 0) scale(${scale})` }}
            />
          </div>
        )}
        <div className="lightbox-caption">
          <div>
            <strong>{photo.title[lang]}</strong>
            <span>{photo.loc[lang]} · {photo.date}</span>
          </div>
          <span>
            {categories.find((item) => item.key === photo.cat)?.[lang]} · {Math.round(scale * 100)}%
          </span>
        </div>
      </div>
    </div>
  );
}
