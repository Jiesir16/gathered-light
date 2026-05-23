// 前台 App
const { useState: u, useEffect: e, useMemo: m, useCallback: c } = React;

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "layout": "masonry",
  "density": "default",
  "background": "warm",
  "accent": "ink",
  "showCaptionsAlways": false
}/*EDITMODE-END*/;

function Tweaks({ t, setTweak, i18n }) {
  return (
    <TweaksPanel title={i18n.panelTitle}>
      <TweakSection title={i18n.layout}>
        <TweakRadio label={i18n.layout} value={t.layout} options={[{value:"masonry",label:i18n.masonry},{value:"grid",label:i18n.grid}]} onChange={(v) => setTweak("layout", v)} />
        <TweakRadio label={i18n.density} value={t.density} options={[{value:"tight",label:i18n.tight},{value:"default",label:i18n.default},{value:"spacious",label:i18n.spacious}]} onChange={(v) => setTweak("density", v)} />
      </TweakSection>
      <TweakSection title={i18n.bg}>
        <TweakRadio label={i18n.bg} value={t.background} options={[{value:"pure",label:i18n.pure},{value:"warm",label:i18n.warm},{value:"cool",label:i18n.cool}]} onChange={(v) => setTweak("background", v)} />
        <TweakColor label={i18n.accent} value={t.accent} options={[
          {value:"ink", color:"#1a1a1a"},
          {value:"sienna", color:"#9a5a3c"},
          {value:"sage", color:"#5e7a64"},
          {value:"slate", color:"#4a5a78"},
        ]} onChange={(v) => setTweak("accent", v)} />
      </TweakSection>
      <TweakSection title={i18n.caption}>
        <TweakToggle label={i18n.alwaysShow} value={t.showCaptionsAlways} onChange={(v) => setTweak("showCaptionsAlways", v)} />
      </TweakSection>
    </TweaksPanel>
  );
}

function App() {
  const [tw, setTweak] = useTweaks(TWEAK_DEFAULTS);
  const [lang, setLang, i18n] = useLang();
  const photos = usePhotos();
  const [category, setCategory] = u("all");
  const [activeId, setActiveId] = u(null);
  const [unlockedIds, setUnlockedIds] = u(() => new Set());

  const filtered = m(() => category === "all" ? photos : photos.filter((p) => p.cat === category), [category, photos]);
  const activePhoto = m(() => filtered.find((p) => p.id === activeId) || null, [filtered, activeId]);
  const activeIdx = m(() => filtered.findIndex((p) => p.id === activeId), [filtered, activeId]);

  const onPrev = c(() => { if (filtered.length === 0) return; const i = activeIdx <= 0 ? filtered.length - 1 : activeIdx - 1; setActiveId(filtered[i].id); }, [filtered, activeIdx]);
  const onNext = c(() => { if (filtered.length === 0) return; const i = (activeIdx + 1) % filtered.length; setActiveId(filtered[i].id); }, [filtered, activeIdx]);
  const onUnlock = (id) => setUnlockedIds((p) => { const n = new Set(p); n.add(id); return n; });

  const toneVars = {
    pure: { bg:"#FFFFFF", card:"#FAFAFA", fg:"#0E0E10", muted2:"#5b5b60", muted:"#9a9a9f", rule:"#ececec" },
    warm: { bg:"#FAF8F4", card:"#F4F1EA", fg:"#1A1814", muted2:"#5d574d", muted:"#9b958a", rule:"#e8e3d8" },
    cool: { bg:"#F7F8FA", card:"#EEF0F4", fg:"#11141a", muted2:"#525762", muted:"#8c919c", rule:"#e3e6ec" },
  }[tw.background];
  const accentColor = { ink:"#1a1a1a", sienna:"#9a5a3c", sage:"#5e7a64", slate:"#4a5a78" }[tw.accent] || "#1a1a1a";

  e(() => {
    const r = document.documentElement;
    r.style.setProperty("--bg", toneVars.bg);
    r.style.setProperty("--card", toneVars.card);
    r.style.setProperty("--fg", toneVars.fg);
    r.style.setProperty("--muted", toneVars.muted);
    r.style.setProperty("--muted-2", toneVars.muted2);
    r.style.setProperty("--rule", toneVars.rule);
    r.style.setProperty("--accent", accentColor);
  }, [tw.background, tw.accent]);

  return (
    <div id="top" className="min-h-screen" style={{ background: "var(--bg)", color: "var(--fg)" }}>
      <style>{`
        :root { --bg:#FAF8F4; --card:#F4F1EA; --fg:#1A1814; --muted:#9b958a; --muted-2:#5d574d; --rule:#e8e3d8; --accent:#1a1a1a; }
        body { font-family: 'Noto Sans SC', 'Inter', ui-sans-serif, system-ui, -apple-system, sans-serif; -webkit-font-smoothing: antialiased; }
        .font-serif { font-family: 'Noto Serif SC', 'Cormorant Garamond', 'Source Han Serif SC', 'Songti SC', Georgia, serif; font-weight: 500; }
        .no-scrollbar::-webkit-scrollbar { display: none; }
        .no-scrollbar { scrollbar-width: none; -ms-overflow-style: none; }
        @keyframes fadeIn { from { opacity: 0 } to { opacity: 1 } }
        ${tw.showCaptionsAlways ? "figure figcaption { opacity: 1 !important; }" : ""}
        @media (hover: none) {
          figure figcaption { opacity: 1 !important; }
          figure img { transform: none !important; }
        }
      `}</style>

      <Header category={category} setCategory={setCategory} accent={accentColor} lang={lang} setLang={setLang} t={i18n} />
      <Intro count={photos.length} t={i18n} />
      <Gallery photos={filtered} onOpen={(p) => setActiveId(p.id)} unlockedIds={unlockedIds} layout={tw.layout} density={tw.density} lang={lang} t={i18n} />
      <Footer t={i18n} />
      <Lightbox photo={activePhoto} onClose={() => setActiveId(null)} onPrev={onPrev} onNext={onNext} unlockedIds={unlockedIds} onUnlock={onUnlock} lang={lang} t={i18n} />
      <Tweaks t={tw} setTweak={setTweak} i18n={i18n} />
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(<App />);
