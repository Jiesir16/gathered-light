// 共享组件 — Header, Lightbox, PhotoCard, Gallery, Footer
const { useState, useEffect, useMemo, useRef, useCallback } = React;

// 图标
const LockIcon = ({ size = 12 }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
    <rect x="4" y="11" width="16" height="10" rx="1.5" />
    <path d="M8 11V7a4 4 0 0 1 8 0v4" />
  </svg>
);
const EyeOffIcon = ({ size = 12 }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
    <path d="M3 3l18 18" />
    <path d="M10.6 10.6a2 2 0 0 0 2.8 2.8" />
    <path d="M9.9 5.1A10 10 0 0 1 12 5c5 0 9 4 10 7-0.4 1.1-1.2 2.4-2.3 3.6M6.3 6.3C4.4 7.6 3 9.6 2 12c1 3 5 7 10 7 1.7 0 3.3-.4 4.7-1.1" />
    <path d="M14 14l-4-4" />
  </svg>
);
const CloseIcon = ({ size = 22 }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round">
    <path d="M6 6l12 12M18 6L6 18" />
  </svg>
);
const ArrowIcon = ({ dir = "right", size = 22 }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" style={{ transform: dir === "left" ? "scaleX(-1)" : "none" }}>
    <path d="M5 12h14M13 6l6 6-6 6" />
  </svg>
);

// Header
function Header({ category, setCategory, accent, lang, setLang, t }) {
  return (
    <header className="sticky top-0 z-30 backdrop-blur-md bg-[var(--bg)]/80 border-b border-[var(--rule)]">
      <div className="max-w-[1480px] mx-auto px-5 sm:px-8 lg:px-14 h-16 sm:h-20 flex items-center justify-between gap-3">
        <a href="#top" className="flex items-baseline gap-3 group shrink-0">
          <span className="font-serif text-[22px] sm:text-[26px] tracking-tight text-[var(--fg)] leading-none">{t.siteName}</span>
          <span className="hidden sm:inline-block text-[10px] tracking-[0.28em] uppercase text-[var(--muted)] mb-[2px]">{t.siteSub}</span>
        </a>

        <nav className="flex items-center gap-0 sm:gap-1 overflow-x-auto no-scrollbar -mx-2 px-2">
          {CATEGORIES.map((c) => {
            const active = c.key === category;
            return (
              <button key={c.key} onClick={() => setCategory(c.key)}
                className="relative shrink-0 px-3 sm:px-4 py-2 text-[13px] sm:text-[14px] tracking-[0.08em] sm:tracking-[0.12em] transition-colors duration-300"
                style={{ color: active ? "var(--fg)" : "var(--muted)" }}
                onMouseEnter={(e) => { if (!active) e.currentTarget.style.color = "var(--fg)"; }}
                onMouseLeave={(e) => { if (!active) e.currentTarget.style.color = "var(--muted)"; }}
              >
                {c[lang]}
                <span className="absolute left-2 right-2 sm:left-3 sm:right-3 -bottom-[1px] h-px transition-all duration-500 ease-out"
                  style={{ background: accent, transform: active ? "scaleX(1)" : "scaleX(0)", transformOrigin: "left", opacity: active ? 1 : 0 }} />
              </button>
            );
          })}
        </nav>

        <div className="flex items-center gap-3 sm:gap-4 shrink-0">
          {/* Lang toggle */}
          <div className="flex items-center text-[11px] tracking-[0.16em] border border-[var(--rule)] rounded-full overflow-hidden">
            <button onClick={() => setLang("zh")} className="px-2.5 py-1 transition-colors" style={{ background: lang === "zh" ? "var(--fg)" : "transparent", color: lang === "zh" ? "var(--bg)" : "var(--muted)" }}>中</button>
            <button onClick={() => setLang("en")} className="px-2.5 py-1 transition-colors" style={{ background: lang === "en" ? "var(--fg)" : "transparent", color: lang === "en" ? "var(--bg)" : "var(--muted)" }}>EN</button>
          </div>
          <a href="admin.html" className="hidden sm:inline-block text-[11px] tracking-[0.18em] uppercase text-[var(--muted)] hover:text-[var(--fg)] transition-colors">{t.admin}</a>
          <div className="hidden lg:flex items-center gap-4 text-[11px] tracking-[0.18em] uppercase text-[var(--muted)] whitespace-nowrap">
            <span className="w-px h-3 bg-[var(--rule)]" />
            <span>{t.range}</span>
          </div>
        </div>
      </div>
    </header>
  );
}

// 引言
function Intro({ count, t }) {
  return (
    <section className="max-w-[1480px] mx-auto px-5 sm:px-8 lg:px-14 pt-10 sm:pt-20 pb-10 sm:pb-16">
      <div className="grid grid-cols-12 gap-6 sm:gap-8 items-end">
        <div className="col-span-12 lg:col-span-8">
          <p className="text-[10px] tracking-[0.32em] uppercase text-[var(--muted)] mb-4 sm:mb-6">{t.issue}</p>
          <h1 className="font-serif text-[clamp(24px,3.2vw,40px)] leading-[1.55] tracking-[0.005em] text-[var(--fg)] font-normal max-w-[760px]">
            {t.headline1}<span className="italic text-[var(--muted-2)]">{t.headline2}</span>{t.headline3}{t.headline4}{t.headline5}
          </h1>
        </div>
        <div className="col-span-12 lg:col-span-4 lg:pl-8">
          <p className="text-[14px] leading-[1.9] text-[var(--muted-2)] max-w-sm">{t.intro}</p>
          <div className="mt-8 flex items-center gap-6 text-[11px] tracking-[0.16em] uppercase text-[var(--muted)] whitespace-nowrap">
            <span>{t.frames(count)}</span>
            <span className="w-6 h-px bg-[var(--rule)]" />
            <span>{t.updated}</span>
          </div>
        </div>
      </div>
    </section>
  );
}

// 卡片
function PhotoCard({ photo, onOpen, unlocked, layout, lang, t }) {
  const isLocked = photo.privacy === "locked" && !unlocked;
  const isPrivate = photo.privacy === "private";
  return (
    <figure className={"group relative cursor-pointer overflow-hidden bg-[var(--card)] " + (layout === "masonry" ? "mb-3 sm:mb-5 break-inside-avoid" : "")}
      onClick={() => onOpen(photo)}>
      <div className="relative overflow-hidden">
        <img src={photo.src} alt={photo.title[lang]} loading="lazy"
          className={"w-full h-auto block transition-all duration-[600ms] ease-out group-hover:scale-[1.035] " + (isLocked || isPrivate ? "blur-xl scale-110" : "")}
          style={layout === "grid" ? { aspectRatio: "4/5", objectFit: "cover" } : {}}
        />
        <div className="absolute inset-0 bg-black/0 group-hover:bg-black/[0.04] transition-colors duration-500" />
        {(isLocked || isPrivate) && (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 text-white/95">
            <div className="w-10 h-10 rounded-full bg-white/12 backdrop-blur-sm flex items-center justify-center border border-white/25">
              {isLocked ? <LockIcon size={16} /> : <EyeOffIcon size={16} />}
            </div>
            <p className="text-[11px] tracking-[0.32em]">{isLocked ? t.locked : t.private}</p>
          </div>
        )}
      </div>
      <figcaption className="absolute left-0 right-0 bottom-0 p-4 flex items-end justify-between gap-3 opacity-0 group-hover:opacity-100 transition-opacity duration-500 bg-gradient-to-t from-black/55 via-black/15 to-transparent text-white">
        <div>
          <div className="font-serif text-[20px] leading-tight">{photo.title[lang]}</div>
          <div className="text-[10px] tracking-[0.22em] mt-1 opacity-85">{photo.loc[lang]} · {photo.date}</div>
        </div>
        <div className="text-[10px] tracking-[0.22em] opacity-85">{CATEGORIES.find(c => c.key === photo.cat)?.[lang]}</div>
      </figcaption>
      {photo.privacy === "locked" && unlocked && (
        <div className="absolute top-3 right-3 w-6 h-6 rounded-full bg-white/85 backdrop-blur text-[var(--fg)] flex items-center justify-center"><LockIcon size={10} /></div>
      )}
    </figure>
  );
}

// 画廊
function Gallery({ photos, onOpen, unlockedIds, layout, density, lang, t }) {
  const cols = density === "tight" ? "columns-2 md:columns-3 lg:columns-4 xl:columns-4"
    : density === "spacious" ? "columns-1 md:columns-2 lg:columns-3"
    : "columns-2 md:columns-3 lg:columns-3 xl:columns-4";
  if (layout === "grid") {
    return (
      <section className="max-w-[1480px] mx-auto px-5 sm:px-8 lg:px-14 pb-20 sm:pb-32">
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3 sm:gap-5">
          {photos.map((p) => <PhotoCard key={p.id} photo={p} onOpen={onOpen} unlocked={unlockedIds.has(p.id)} layout="grid" lang={lang} t={t} />)}
        </div>
      </section>
    );
  }
  return (
    <section className="max-w-[1480px] mx-auto px-5 sm:px-8 lg:px-14 pb-20 sm:pb-32">
      <div className={`${cols} gap-3 sm:gap-5`}>
        {photos.map((p) => <PhotoCard key={p.id} photo={p} onOpen={onOpen} unlocked={unlockedIds.has(p.id)} layout="masonry" lang={lang} t={t} />)}
      </div>
    </section>
  );
}

// Lightbox
const UNLOCK_PASSCODE = "1234";
function Lightbox({ photo, onClose, onPrev, onNext, unlockedIds, onUnlock, lang, t }) {
  const [code, setCode] = useState("");
  const [error, setError] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const dragRef = useRef(null);

  useEffect(() => { setCode(""); setError(false); setZoom(1); setPan({x:0,y:0}); }, [photo?.id]);

  const ZOOM_MIN = 1, ZOOM_MAX = 4;
  const clampZoom = (z) => Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, z));
  const zoomIn = () => setZoom((z) => { const nz = clampZoom(z + 0.5); if (nz === 1) setPan({x:0,y:0}); return nz; });
  const zoomOut = () => setZoom((z) => { const nz = clampZoom(z - 0.5); if (nz === 1) setPan({x:0,y:0}); return nz; });
  const zoomReset = () => { setZoom(1); setPan({x:0,y:0}); };
  const onWheel = (e) => { e.preventDefault(); setZoom((z) => { const nz = clampZoom(z + (e.deltaY < 0 ? 0.25 : -0.25)); if (nz === 1) setPan({x:0,y:0}); return nz; }); };
  const onDoubleClick = () => { if (zoom > 1) zoomReset(); else setZoom(2); };
  const onPointerDown = (e) => { if (zoom <= 1) return; dragRef.current = { startX: e.clientX, startY: e.clientY, panX: pan.x, panY: pan.y }; e.currentTarget.setPointerCapture(e.pointerId); };
  const onPointerMove = (e) => { if (!dragRef.current) return; setPan({ x: dragRef.current.panX + (e.clientX - dragRef.current.startX), y: dragRef.current.panY + (e.clientY - dragRef.current.startY) }); };
  const onPointerUp = () => { dragRef.current = null; };

  useEffect(() => {
    if (!photo) return;
    const onKey = (e) => {
      if (e.key === "Escape") onClose();
      else if (e.key === "ArrowLeft") onPrev();
      else if (e.key === "ArrowRight") onNext();
      else if (e.key === "+" || e.key === "=") zoomIn();
      else if (e.key === "-" || e.key === "_") zoomOut();
      else if (e.key === "0") zoomReset();
    };
    window.addEventListener("keydown", onKey);
    document.body.style.overflow = "hidden";
    return () => { window.removeEventListener("keydown", onKey); document.body.style.overflow = ""; };
  }, [photo, onClose, onPrev, onNext]);

  if (!photo) return null;
  const isLocked = photo.privacy === "locked" && !unlockedIds.has(photo.id);
  const isPrivate = photo.privacy === "private";
  const tryUnlock = async () => {
    if (await Store.unlock(photo.id, code)) { onUnlock(photo.id); setError(false); }
    else { setError(true); setTimeout(() => setError(false), 1400); }
  };
  const privacyLabel = photo.privacy === "public" ? t.public : photo.privacy === "locked" ? (unlockedIds.has(photo.id) ? t.unlocked : t.locked) : t.private;
  const catLabel = CATEGORIES.find(c => c.key === photo.cat)?.[lang] || "";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-6 md:p-10 animate-[fadeIn_240ms_ease-out]"
      style={{ background: "rgba(12,12,14,0.78)", backdropFilter: "blur(18px)", WebkitBackdropFilter: "blur(18px)" }}
      onClick={onClose}>
      <button onClick={onClose} className="absolute top-3 right-3 sm:top-6 sm:right-6 md:top-8 md:right-8 w-11 h-11 rounded-full flex items-center justify-center text-white/90 hover:text-white bg-black/30 sm:bg-transparent sm:hover:bg-white/10 transition-colors z-10" aria-label={t.close}><CloseIcon /></button>
      <button onClick={(e) => { e.stopPropagation(); onPrev(); }} className="hidden sm:flex absolute left-4 md:left-8 top-1/2 -translate-y-1/2 w-11 h-11 rounded-full items-center justify-center text-white/80 hover:text-white hover:bg-white/10 transition-colors" aria-label={t.prev}><ArrowIcon dir="left" /></button>
      <button onClick={(e) => { e.stopPropagation(); onNext(); }} className="hidden sm:flex absolute right-4 md:right-8 top-1/2 -translate-y-1/2 w-11 h-11 rounded-full items-center justify-center text-white/80 hover:text-white hover:bg-white/10 transition-colors" aria-label={t.next}><ArrowIcon dir="right" /></button>

      <div className="relative w-full max-w-[92vw] max-h-[92vh] flex flex-col items-center" onClick={(e) => e.stopPropagation()}>
        <div className="relative max-w-[94vw] max-h-[68vh] sm:max-h-[74vh] flex items-center justify-center">
          {isPrivate ? (
            <div className="px-12 py-16 border border-white/15 text-white/90 text-center">
              <div className="w-12 h-12 rounded-full bg-white/10 mx-auto flex items-center justify-center mb-4"><EyeOffIcon size={20} /></div>
              <div className="font-serif text-2xl mb-2">{t.privateTitle}</div>
              <div className="text-[11px] tracking-[0.24em] opacity-70">{t.privateSub}</div>
            </div>
          ) : isLocked ? (
            <div className="px-12 py-14 border border-white/15 text-white/95 text-center w-[min(420px,90vw)]">
              <div className="w-12 h-12 rounded-full bg-white/10 mx-auto flex items-center justify-center mb-5"><LockIcon size={18} /></div>
              <div className="font-serif text-2xl mb-2">{photo.title[lang]}</div>
              <p className="text-[12px] tracking-[0.04em] opacity-70 mb-6 leading-relaxed">{t.lockedHint}</p>
              <div className="flex flex-col items-center gap-3">
                <input type="password" value={code} onChange={(e) => setCode(e.target.value)} onKeyDown={(e) => e.key === "Enter" && tryUnlock()} placeholder={t.passcode}
                  className={"w-full bg-transparent border-b text-center tracking-[0.4em] py-2 text-white placeholder:text-white/30 outline-none transition-colors " + (error ? "border-red-300" : "border-white/30 focus:border-white/80")} />
                <button onClick={tryUnlock} className="mt-2 px-6 py-2 text-[12px] tracking-[0.28em] border border-white/40 hover:bg-white hover:text-black transition-colors">{t.unlock}</button>
                <p className="text-[10px] tracking-[0.18em] opacity-50 mt-2">{t.hint}</p>
              </div>
            </div>
          ) : (
            <div className="overflow-hidden flex items-center justify-center" onWheel={onWheel} onDoubleClick={onDoubleClick}
              onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={onPointerUp} onPointerCancel={onPointerUp}
              style={{ cursor: zoom > 1 ? (dragRef.current ? "grabbing" : "grab") : "zoom-in", touchAction: zoom > 1 ? "none" : "auto" }}>
              <img src={photo.src.replace(/w=\d+/, "w=1800")} alt={photo.title[lang]} draggable={false}
                className="max-w-[94vw] max-h-[68vh] sm:max-h-[74vh] object-contain shadow-[0_30px_80px_-20px_rgba(0,0,0,0.6)] select-none"
                style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`, transition: dragRef.current ? "none" : "transform 220ms ease-out" }} />
            </div>
          )}
        </div>

        {!isPrivate && !isLocked && (
          <div className="mt-3 flex items-center gap-1 px-1 py-1 rounded-full bg-white/10 backdrop-blur-sm border border-white/15 text-white/90">
            <button onClick={(e) => { e.stopPropagation(); zoomOut(); }} disabled={zoom <= 1} className="w-8 h-8 rounded-full flex items-center justify-center hover:bg-white/15 disabled:opacity-30 transition-colors" aria-label={t.zoomOut}><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round"><path d="M5 12h14"/></svg></button>
            <button onClick={(e) => { e.stopPropagation(); zoomReset(); }} className="px-3 h-8 text-[11px] tracking-[0.16em] hover:bg-white/15 rounded-full transition-colors min-w-[56px]" aria-label={t.zoomReset}>{Math.round(zoom*100)}%</button>
            <button onClick={(e) => { e.stopPropagation(); zoomIn(); }} disabled={zoom >= 4} className="w-8 h-8 rounded-full flex items-center justify-center hover:bg-white/15 disabled:opacity-30 transition-colors" aria-label={t.zoomIn}><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round"><path d="M12 5v14M5 12h14"/></svg></button>
          </div>
        )}

        <div className="mt-3 sm:mt-4 w-full max-w-[760px] flex items-end justify-between text-white px-2 gap-3">
          <div className="min-w-0">
            <div className="font-serif text-[18px] sm:text-[24px] leading-tight truncate">{photo.title[lang]}</div>
            <div className="text-[10px] sm:text-[11px] tracking-[0.22em] sm:tracking-[0.28em] mt-1 sm:mt-1.5 text-white/60 truncate">{photo.loc[lang]} · {photo.date}</div>
          </div>
          <div className="text-[10px] sm:text-[11px] tracking-[0.22em] sm:tracking-[0.28em] text-white/55 text-right shrink-0">
            <div>{catLabel}</div>
            <div className="mt-1">{privacyLabel}</div>
          </div>
        </div>

        <div className="sm:hidden mt-3 flex items-center gap-3">
          <button onClick={(e) => { e.stopPropagation(); onPrev(); }} className="w-12 h-12 rounded-full flex items-center justify-center text-white/85 bg-white/10 active:bg-white/20"><ArrowIcon dir="left" size={20} /></button>
          <button onClick={(e) => { e.stopPropagation(); onNext(); }} className="w-12 h-12 rounded-full flex items-center justify-center text-white/85 bg-white/10 active:bg-white/20"><ArrowIcon dir="right" size={20} /></button>
        </div>
      </div>
    </div>
  );
}

// Footer
function Footer({ t }) {
  return (
    <footer className="border-t border-[var(--rule)]">
      <div className="max-w-[1480px] mx-auto px-5 sm:px-8 lg:px-14 py-10 sm:py-14 flex flex-col md:flex-row md:items-end md:justify-between gap-8">
        <div>
          <div className="font-serif text-2xl text-[var(--fg)]">{t.siteName}</div>
          <p className="text-[12px] text-[var(--muted)] mt-2 max-w-xs leading-relaxed">{t.footerNote}</p>
        </div>
        <div className="grid grid-cols-2 gap-x-12 gap-y-3 text-[12px] tracking-[0.18em] text-[var(--muted)]">
          <a className="hover:text-[var(--fg)] transition-colors" href="#">Instagram</a>
          <a className="hover:text-[var(--fg)] transition-colors" href="#">VSCO</a>
          <a className="hover:text-[var(--fg)] transition-colors" href="#">Mail</a>
          <a className="hover:text-[var(--fg)] transition-colors" href="admin.html">{t.admin}</a>
        </div>
        <div className="text-[11px] tracking-[0.24em] text-[var(--muted)]">{t.edition}</div>
      </div>
    </footer>
  );
}

Object.assign(window, { Header, Intro, PhotoCard, Gallery, Lightbox, Footer, LockIcon, EyeOffIcon, CloseIcon, ArrowIcon });
