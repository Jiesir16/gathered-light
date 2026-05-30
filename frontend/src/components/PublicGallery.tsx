import {
  type CSSProperties,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
  type WheelEvent as ReactWheelEvent
} from "react";
import { api, settings } from "../api/client";
import type { HeroCopy } from "../api/client";
import { categories } from "../i18n";
import { useLang } from "../hooks/useLang";
import { usePhotos } from "../hooks/usePhotos";
import type { Photo } from "../types";
import {
  ArrowIcon,
  CloseIcon,
  DownloadIcon,
  ExternalLinkIcon,
  EyeOffIcon,
  InfoIcon,
  LockIcon,
  RotateIcon,
  ZoomInIcon,
  ZoomOutIcon
} from "./Icons";

export function PublicGallery() {
  const { lang, setLang, t } = useLang();
  const { photos, loading, error } = usePhotos(false);
  const [category, setCategory] = useState("all");
  const [activeId, setActiveId] = useState<number | null>(null);
  const [unlocked, setUnlocked] = useState<Set<number>>(() => new Set());
  const [hero, setHero] = useState<HeroCopy | null>(null);
  const masonryRef = useRef<HTMLElement | null>(null);

  const filtered = useMemo(() => (
    category === "all" ? photos : photos.filter((photo) => photo.cat === category)
  ), [category, photos]);
  const issue = hero ? hero.issue[lang] : (t.issue as string);
  const headline = hero ? hero.headline[lang] : (t.headline as string);
  const introLines = hero ? hero.introLines[lang] : t.introLines;
  const active = filtered.find((photo) => photo.id === activeId) ?? null;
  const activeIndex = filtered.findIndex((photo) => photo.id === activeId);

  useEffect(() => {
    let alive = true;
    settings
      .get()
      .then((resp) => {
        if (!alive) return;
        if (resp.hero) setHero(resp.hero);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, []);

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
    cards.forEach((card) => {
      card.classList.remove("is-visible");
      card.style.setProperty("--reveal-delay", "0ms");
    });

    let revealFrame = 0;

    revealFrame = window.requestAnimationFrame(() => {
      const topValues = cards.map((card) => card.offsetTop);
      const minTop = Math.min(...topValues);
      cards.forEach((card, index) => {
        const delay = Math.min(120, Math.max(0, (topValues[index] - minTop) * 0.04));
        card.style.setProperty("--reveal-delay", `${Math.round(delay)}ms`);
      });

      revealFrame = window.requestAnimationFrame(() => {
        cards.forEach((card) => card.classList.add("is-visible"));
      });
    });

    return () => {
      window.cancelAnimationFrame(revealFrame);
    };
  }, [category, filtered]);

  return (
    <div id="top" className="site-shell">
      <header className="topbar">
        <div className="topbar-inner">
          <a className="brand" href="#top">
            <span className="brand-mark">{t.siteName}</span>
            <small>{t.siteSub}</small>
          </a>
          <nav className="category-nav" aria-label="categories">
            {categories.map((item) => (
              <button
                key={item.key}
                type="button"
                className={category === item.key ? "active" : ""}
                aria-pressed={category === item.key}
                onClick={() => setCategory(item.key)}
              >
                {item[lang]}
              </button>
            ))}
          </nav>
          <div className="top-actions">
            <div className="lang-switch">
              <button className={lang === "zh" ? "active" : ""} onClick={() => setLang("zh")}>中</button>
              <button className={lang === "en" ? "active" : ""} onClick={() => setLang("en")}>EN</button>
            </div>
          </div>
        </div>
      </header>

      <main>
        <section className="intro">
          <div className="intro-copy">
            <p className="intro-kicker">{issue}</p>
            <h1 className="intro-title">{headline}</h1>
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
          {filtered.map((photo, index) => (
            <PhotoCard
              key={photo.id}
              photo={photo}
              index={index}
              priority={index < 8}
              unlocked={unlocked.has(photo.id)}
              onOpen={() => setActiveId(photo.id)}
              lang={lang}
            />
          ))}
        </section>
      </main>

      <footer className="footer">
        <div className="footer-content">
          <div className="footer-copy">
            <strong className="footer-brand">{t.siteName}</strong>
            <p>{t.footerNote as string}</p>
          </div>
          <nav className="footer-links" aria-label={lang === "zh" ? "外部链接" : "External links"}>
            <a href="#top">Instagram</a>
            <a href="#top">VSCO</a>
            <a href="#top">Mail</a>
          </nav>
          <div className="footer-edition">© 2026 · {t.siteName}</div>
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
      ? 54
      : atEnd
        ? 6000
        : atStart && lineIndex > 0
          ? 420
          : 96;

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

const fallbackRatios = ["4 / 5", "3 / 4", "1 / 1", "5 / 4", "2 / 3", "16 / 11", "4 / 3", "5 / 7"];

function photoMediaStyle(photo: Photo, index: number): CSSProperties {
  if (photo.width && photo.height) {
    return { aspectRatio: `${photo.width} / ${photo.height}` };
  }
  return { aspectRatio: fallbackRatios[index % fallbackRatios.length] };
}

function PhotoCard({
  photo,
  index,
  priority,
  unlocked,
  onOpen,
  lang
}: {
  photo: Photo;
  index: number;
  priority: boolean;
  unlocked: boolean;
  onOpen: () => void;
  lang: "zh" | "en";
}) {
  const hidden = photo.privacy === "private" || (photo.privacy === "locked" && !unlocked);
  // 优先 webp（现代浏览器自动选）→ medium JPG → 兜底 src
  const jpgSrc = photo.variants?.medium ?? photo.src;
  const webpSrc = photo.variants?.webp;
  const fetchPriority = index < 4 ? "high" : "auto";
  return (
    <figure
      className="photo-card"
      onClick={(event) => {
        event.stopPropagation();
        onOpen();
      }}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onOpen();
        }
      }}
      role="button"
      tabIndex={0}
    >
      <div className="photo-media" style={photoMediaStyle(photo, index)}>
        {!hidden && (
          <picture>
            {webpSrc && <source srcSet={webpSrc} type="image/webp" />}
            <img
              src={jpgSrc}
              alt={photo.title[lang]}
              // 原始宽高让浏览器在加载前按比例预留版位，避免瀑布流回流抖动（CLS）
              width={photo.width}
              height={photo.height}
              loading={priority ? "eager" : "lazy"}
              decoding="async"
              // data-loaded 触发淡入；缓存命中也会 fire onLoad，所以无 flicker
              ref={(image) => {
                image?.setAttribute("fetchpriority", fetchPriority);
                if (image?.complete && image.naturalWidth > 0) image.dataset.loaded = "true";
              }}
              onLoad={(event) => { event.currentTarget.dataset.loaded = "true"; }}
              onError={(event) => { event.currentTarget.dataset.error = "true"; }}
            />
          </picture>
        )}
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
  const [rotation, setRotation] = useState(0);
  const [showInfo, setShowInfo] = useState(false);
  const [originalLoading, setOriginalLoading] = useState(false);
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
    setRotation(0);
    setShowInfo(false);
    setOriginalLoading(false);
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
        setRotation(0);
        setOffset({ x: 0, y: 0 });
      } else if (event.key === "r" || event.key === "R") {
        setRotation((value) => value + 90);
        setOffset({ x: 0, y: 0 });
      } else if (event.key === "l" || event.key === "L") {
        setRotation((value) => value - 90);
        setOffset({ x: 0, y: 0 });
      } else if (event.key === "i" || event.key === "I") {
        setShowInfo((value) => !value);
      }
    };

    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      document.body.style.overflow = previousOverflow;
    };
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
  const categoryLabel = categories.find((item) => item.key === photo.cat)?.[lang] ?? photo.cat;
  const privacyLabel = photo.privacy === "public"
    ? t.public as string
    : photo.privacy === "locked"
      ? (unlocked ? t.unlocked as string : t.locked as string)
      : t.private as string;
  const detailRows = getPhotoDetailRows(photo, lang, categoryLabel, privacyLabel);

  const resetView = () => {
    setScale(1);
    setRotation(0);
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
    setImageQuality((value) => {
      const next = value === "original" ? "full" : "original";
      setOriginalLoading(next === "original");
      return next;
    });
    resetView();
  };

  const rotateBy = (degrees: -90 | 90) => {
    setRotation((value) => value + degrees);
    setOffset({ x: 0, y: 0 });
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
      <button className="icon-button prev" onClick={(event) => { event.stopPropagation(); onPrev(); }} aria-label={t.prev as string}><ArrowIcon direction="left" /></button>
      <button className="icon-button next" onClick={(event) => { event.stopPropagation(); onNext(); }} aria-label={t.next as string}><ArrowIcon /></button>
      <div className="lightbox-inner" onClick={(event) => event.stopPropagation()}>
        <div className="lightbox-viewer">
          {privatePhoto ? (
            <div className="locked-panel">
              <EyeOffIcon size={20} />
              <h2>{t.privateTitle as string}</h2>
              <p>{t.privateOnly as string}</p>
            </div>
          ) : locked ? (
            <div className="locked-panel">
              <LockIcon size={18} />
              <h2>{photo.title[lang]}</h2>
              <p>{t.lockedHint as string}</p>
              <div className="locked-panel-form">
                <input value={passcode} onChange={(event) => setPasscode(event.target.value)} onKeyDown={(event) => event.key === "Enter" && void unlock()} placeholder={t.passcode as string} type="password" />
                <button className="primary" onClick={() => void unlock()}>{t.unlock as string}</button>
                <p className="locked-panel-hint">{t.lockedPasscodeHint as string}</p>
                {error && <span className="form-error">{error}</span>}
              </div>
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
                draggable={false}
                ref={(image) => image?.setAttribute("fetchpriority", "high")}
                onLoad={() => setOriginalLoading(false)}
                onError={() => setOriginalLoading(false)}
                style={{ transform: `translate3d(${offset.x}px, ${offset.y}px, 0) rotate(${rotation}deg) scale(${scale})` }}
              />
              {originalLoading && (
                <div className="lightbox-loading" aria-live="polite">
                  <span aria-hidden="true" />
                  {t.loadingOriginal as string}
                </div>
              )}
            </div>
          )}
        </div>
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
              className="viewer-reset"
              onClick={resetView}
              aria-label={t.fitToScreen as string}
              title={t.fitToScreen as string}
            >
              {Math.round(scale * 100)}%
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
            <span className="lightbox-action-sep" aria-hidden="true" />
            <button
              className="icon-button"
              onClick={() => rotateBy(-90)}
              aria-label={t.rotateLeft as string}
              title={t.rotateLeft as string}
            >
              <RotateIcon direction="left" size={17} />
            </button>
            <button
              className="icon-button"
              onClick={() => rotateBy(90)}
              aria-label={t.rotateRight as string}
              title={t.rotateRight as string}
            >
              <RotateIcon size={17} />
            </button>
            <span className="lightbox-action-sep" aria-hidden="true" />
            <button
              className={`icon-button${showInfo ? " active" : ""}`}
              onClick={() => setShowInfo((value) => !value)}
              aria-label={t.photoInfo as string}
              title={t.photoInfo as string}
            >
              <InfoIcon size={17} />
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
            <a
              className="icon-button"
              href={originalUrl ?? fullUrl}
              target="_blank"
              rel="noreferrer"
              onClick={(event) => event.stopPropagation()}
              aria-label={t.newTab as string}
              title={t.newTab as string}
            >
              <ExternalLinkIcon size={17} />
            </a>
          </div>
        )}
        {showInfo && !privatePhoto && !locked && (
          <div className="lightbox-info">
            <div className="lightbox-info-heading">
              <span>{t.photoInfo as string}</span>
              <strong>{photo.caption?.[lang] || photo.alt_text?.[lang] || (t.noPhotoInfo as string)}</strong>
            </div>
            <div className="lightbox-info-grid">
              {detailRows.map((row) => (
                <div key={row.label}>
                  <span>{row.label}</span>
                  <strong>{row.value}</strong>
                </div>
              ))}
            </div>
          </div>
        )}
        <div className="lightbox-caption">
          <div>
            <strong>{photo.title[lang]}</strong>
            <span>{photo.loc[lang]} · {photo.date}</span>
          </div>
          <div className="lightbox-caption-meta">
            <span>{categoryLabel}</span>
            <span>{privacyLabel}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function getPhotoDetailRows(photo: Photo, lang: "zh" | "en", categoryLabel: string, privacyLabel: string) {
  const tagNames = photo.tags?.map((tag) => tag.name[lang]).filter(Boolean).join(" / ");
  return [
    { label: lang === "zh" ? "分类" : "Category", value: categoryLabel },
    { label: lang === "zh" ? "可见性" : "Privacy", value: privacyLabel },
    { label: lang === "zh" ? "日期" : "Date", value: photo.date },
    { label: "Slug", value: photo.slug || `#${photo.id}` },
    { label: lang === "zh" ? "标签" : "Tags", value: tagNames || (lang === "zh" ? "暂无" : "None") },
  ];
}
