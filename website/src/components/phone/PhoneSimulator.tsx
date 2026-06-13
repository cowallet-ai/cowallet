"use client";

import { useState, useRef, useCallback, useEffect } from "react";
import { useLang } from "@/context/LangContext";
import { PhoneFrame } from "./PhoneFrame";
import { detectIntent } from "@/lib/intents";
import { usePhoneState, type ChatMessage } from "@/hooks/usePhoneState";

export function PhoneSimulator() {
  const { lang } = useLang();
  const {
    state,
    setView,
    setOnboardingStage,
    dismissOnboarding,
    setUserName,
    addMessage,
    clearMessages,
  } = usePhoneState();
  const [inputValue, setInputValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const sendMessage = useCallback(
    (text: string) => {
      if (!text.trim()) return;
      const userMsg: ChatMessage = { id: Date.now().toString(), type: "user", text };
      addMessage(userMsg);
      setInputValue("");
      if (state.currentView !== "chat") setView("chat");

      setTimeout(() => {
        const intent = detectIntent(text, lang);
        const aiMsg: ChatMessage = {
          id: (Date.now() + 1).toString(),
          type: "ai",
          text: intent
            ? (lang === "zh" ? "让我确认一下…" : "Let me make sure I got this…")
            : (lang === "zh" ? `我没太听懂"${text}"。试试说"我的钱都在哪?"` : `I didn't quite catch "${text}". Try "Where's my money?"`),
          intent: intent ? { kind: intent.kind, title: intent.title, sub: intent.sub, yes: intent.yes, no: intent.no } : null,
        };
        addMessage(aiMsg);
      }, 800);
    },
    [lang, addMessage, setView, state.currentView]
  );

  const handleIntentYes = useCallback(() => {
    const msg: ChatMessage = { id: Date.now().toString(), type: "ai", text: lang === "zh" ? "好，这就办。" : "On it." };
    addMessage(msg);
  }, [lang, addMessage]);

  const handleIntentNo = useCallback(() => {
    const msg: ChatMessage = { id: Date.now().toString(), type: "ai", text: lang === "zh" ? "好，我刚理解错了。能再说一次吗?" : "Got it, I misread. Say it again?" };
    addMessage(msg);
  }, [lang, addMessage]);

  return (
    <PhoneFrame>
      {/* Status Bar */}
      <div className="ps-status-bar">
        <span>9:41</span>
        <div className="right">
          <svg viewBox="0 0 18 18" fill="currentColor"><path d="M9 4a5 5 0 0 0-4.95 4.3.8.8 0 0 0 .58.91l.1.02a.8.8 0 0 0 .91-.6 3.4 3.4 0 0 1 6.72 0 .8.8 0 0 0 1.59-.33A5 5 0 0 0 9 4z"/><circle cx="9" cy="11" r="2"/></svg>
          <svg viewBox="0 0 18 18" fill="currentColor"><path d="M3 7v4h2V7zM7 5v8h2V5zM11 3v12h2V3zM15 1v16h2V1z"/></svg>
        </div>
      </div>

      {/* Onboarding */}
      {state.showOnboarding && (
        <div style={{ position: "absolute", inset: 0, zIndex: 10 }}>
          {/* Hero stage */}
          <div className={`ps-stage ${state.onboardingStage === "hero" ? "active" : ""}`}>
            <div className="ps-onb-body" style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", textAlign: "center", padding: "60px 20px 20px" }}>
              <div style={{ width: 100, height: 100, borderRadius: "50%", background: "radial-gradient(circle at 35% 30%, #ffd4bc 0%, #D97757 45%, #8a3f2a 100%)", marginBottom: 24, animation: "ps-orb-breathe 3.8s ease-in-out infinite" }} />
              <h1 className="ps-onb-h1" style={{ textAlign: "center" }}>
                {lang === "zh" ? "会听你说话的" : "A wallet that actually"}
                <br />
                <em style={{ fontFamily: "'Fraunces', serif", fontStyle: "italic", color: "var(--ps-accent)" }}>
                  {lang === "zh" ? "钱包。" : "listens."}
                </em>
              </h1>
              <p className="ps-onb-sub" style={{ textAlign: "center", maxWidth: 280 }}>
                {lang === "zh"
                  ? "就像给你家请了个管家——你说一句，它就去做。"
                  : "Like hiring a butler for your money — say what you need."}
              </p>
            </div>
            <div className="ps-onb-footer">
              <button className="ps-btn ps-btn-accent" onClick={() => setOnboardingStage("start")}>
                {lang === "zh" ? "开始使用" : "Get started"}
              </button>
            </div>
          </div>

          {/* Start stage - three options */}
          <div className={`ps-stage ${state.onboardingStage === "start" ? "active" : ""}`}>
            <div className="ps-onb-body" style={{ paddingTop: 56 }}>
              <h1 className="ps-onb-h1">{lang === "zh" ? "这是你的第一个钱包吗?" : "Is this your first wallet?"}</h1>
              <p className="ps-onb-sub">{lang === "zh" ? "两分钟就能开好。" : "Takes two minutes."}</p>

              <button className="ps-opt-card" onClick={() => setOnboardingStage("creating")}>
                <div className="ps-opt-icon" style={{ background: "var(--ps-accent)" }}>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"><path d="M12 5v14M5 12h14" /></svg>
                </div>
                <div style={{ flex: 1 }}>
                  <div className="ps-opt-title">
                    {lang === "zh" ? "我是新用户" : "I'm new"}
                    <span className="ps-opt-tag">{lang === "zh" ? "推荐" : "Best"}</span>
                  </div>
                  <div className="ps-opt-desc">{lang === "zh" ? "最简单。钥匙分成三份保管。" : "Easiest. Key split into three."}</div>
                </div>
              </button>

              <button className="ps-opt-card" onClick={() => setOnboardingStage("creating")}>
                <div className="ps-opt-icon" style={{ background: "var(--ps-success)" }}>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round"><circle cx="8" cy="15" r="4"/><path d="M10.85 12.15L21 2M18 5l3 3"/></svg>
                </div>
                <div style={{ flex: 1 }}>
                  <div className="ps-opt-title">{lang === "zh" ? "我已有钱包" : "I have a wallet"}</div>
                  <div className="ps-opt-desc">{lang === "zh" ? "用 12/24 个词导入。" : "Import with recovery words."}</div>
                </div>
              </button>
            </div>
          </div>

          {/* Creating stage */}
          <div className={`ps-stage ${state.onboardingStage === "creating" ? "active" : ""}`}>
            <div className="ps-onb-body" style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", textAlign: "center", paddingTop: 80 }}>
              <div style={{ width: 100, height: 100, borderRadius: "50%", background: "radial-gradient(circle at 35% 30%, #ffe0cb 0%, #D97757 45%, #a04b32 100%)", marginBottom: 24, animation: "ps-orb-breathe 1.8s ease-in-out infinite" }} />
              <h1 className="ps-onb-h1" style={{ textAlign: "center" }}>
                {lang === "zh" ? "正在帮你把钥匙分成三份" : "Splitting your key into three"}
              </h1>
              <p className="ps-onb-sub" style={{ textAlign: "center" }}>
                {lang === "zh" ? "任何一份丢了，剩下两份还能开门。" : "If one is lost, the other two still open the door."}
              </p>
              <CreatingProgress onComplete={() => setOnboardingStage("name")} />
            </div>
          </div>

          {/* Name stage */}
          <div className={`ps-stage ${state.onboardingStage === "name" ? "active" : ""}`}>
            <div className="ps-onb-body" style={{ paddingTop: 80 }}>
              <h1 className="ps-onb-h1">{lang === "zh" ? "我该怎么叫你?" : "What should I call you?"}</h1>
              <p className="ps-onb-sub">{lang === "zh" ? "起个名字就行，不用真名。" : "A nickname works."}</p>
              <input
                type="text"
                defaultValue="Alice"
                onChange={(e) => setUserName(e.target.value)}
                placeholder={lang === "zh" ? "比如 小明 / Alice" : "e.g. Alice"}
                style={{
                  width: "100%", padding: "14px 18px", borderRadius: 14, border: "1px solid var(--ps-line)",
                  fontSize: 16, fontFamily: "inherit", outline: "none", background: "#fff",
                }}
              />
            </div>
            <div className="ps-onb-footer">
              <button className="ps-btn ps-btn-accent" onClick={() => setOnboardingStage("ready")}>
                {lang === "zh" ? "下一步" : "Continue"}
              </button>
            </div>
          </div>

          {/* Ready stage */}
          <div className={`ps-stage ${state.onboardingStage === "ready" ? "active" : ""}`}>
            <div className="ps-onb-body" style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", textAlign: "center", paddingTop: 80 }}>
              <div style={{ width: 80, height: 80, borderRadius: "50%", background: "radial-gradient(circle at 35% 30%, #ffd4bc 0%, #D97757 45%, #8a3f2a 100%)", marginBottom: 20 }} />
              <h1 className="ps-onb-h1" style={{ textAlign: "center" }}>
                {lang === "zh" ? "都搞定了。" : "All set."}
              </h1>
              <p className="ps-onb-sub" style={{ textAlign: "center" }}>
                {lang === "zh" ? "你的钱包已经可以用了。" : "Your wallet is ready."}
              </p>
            </div>
            <div className="ps-onb-footer">
              <button className="ps-btn ps-btn-accent" onClick={dismissOnboarding}>
                {lang === "zh" ? "好，开始用" : "Let's go"} →
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Main App */}
      {!state.showOnboarding && (
        <>
          {/* Header */}
          <div className="ps-app-header">
            <div className="title">
              <span className="orb-mini" />
              <span>cowallet</span>
            </div>
          </div>

          {/* Body */}
          <div className="ps-body">
            {/* Home View */}
            <div className={`ps-view ${state.currentView === "home" ? "active" : ""}`}>
              <div className="ps-home-status">
                <span className="dot" />
                {lang === "zh" ? "一切正常 · 三份钥匙都在" : "All good · all keys safe"}
              </div>
              <h1 className="ps-home-greet">
                {lang === "zh" ? "早上好，" : "Good morning, "}
                <em>{state.userName || "Alice"}</em>
                {lang === "zh" ? "。" : "."}
              </h1>
              <div className="ps-home-balance">
                <div className="ps-home-balance-label">{lang === "zh" ? "你的总资产" : "Your total"}</div>
                <div className="ps-home-balance-amt">$48,280<span style={{ fontSize: "60%", color: "var(--ps-ink-3)" }}>.22</span></div>
              </div>

              <div className="ps-section-label">{lang === "zh" ? "试试跟它说话" : "Try talking to it"}</div>
              {[
                { zh: "我这个月花了多少钱?", en: "How much did I spend this month?" },
                { zh: "我那 5 万块闲着呢", en: "I have $50k just sitting" },
                { zh: "老婆生日，给她转 1000 块", en: "Wife's birthday, send her $1000" },
              ].map((p, i) => (
                <button
                  key={i}
                  onClick={() => sendMessage(p[lang])}
                  style={{
                    display: "flex", alignItems: "center", width: "100%", padding: "11px 14px",
                    background: "#fff", border: "1px solid var(--ps-line)", borderRadius: 14,
                    cursor: "pointer", marginBottom: 8, textAlign: "left", fontFamily: "inherit",
                    fontSize: 13, color: "var(--ps-ink-1)", transition: "border-color 0.1s",
                  }}
                >
                  <span style={{ flex: 1 }}>{p[lang]}</span>
                  <span style={{ color: "var(--ps-accent)" }}>→</span>
                </button>
              ))}
            </div>

            {/* Wallet View */}
            <div className={`ps-view ${state.currentView === "wallet" ? "active" : ""}`}>
              <div style={{ textAlign: "center", padding: "20px 0" }}>
                <div className="ps-home-balance-label">{lang === "zh" ? "总资产" : "Total balance"}</div>
                <div className="ps-home-balance-amt">$48,280<span style={{ fontSize: "60%", color: "var(--ps-ink-3)" }}>.22</span></div>
              </div>
              <div className="ps-section-label">{lang === "zh" ? "你的资产" : "Your money"}</div>
              {[
                { sym: "U", color: "#2775ca", name: { zh: "美元稳定币 (USDC)", en: "US Dollar (USDC)" }, amt: "$28,450" },
                { sym: "Ξ", color: "#627eea", name: { zh: "以太币 (ETH)", en: "Ether (ETH)" }, amt: "$16,830" },
                { sym: "s", color: "#00a79d", name: { zh: "质押以太币 (stETH)", en: "Staked ETH" }, amt: "$3,000" },
              ].map((a, i) => (
                <div key={i} style={{ display: "flex", alignItems: "center", gap: 12, padding: "12px 0", borderBottom: "1px solid var(--ps-line)" }}>
                  <div style={{ width: 36, height: 36, borderRadius: 10, background: a.color, color: "#fff", display: "grid", placeItems: "center", fontWeight: 700, fontFamily: "'Fraunces', serif", fontSize: 15 }}>{a.sym}</div>
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: 13, fontWeight: 500 }}>{a.name[lang]}</div>
                  </div>
                  <div style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: 13, fontWeight: 600 }}>{a.amt}</div>
                </div>
              ))}
            </div>

            {/* Agents View */}
            <div className={`ps-view ${state.currentView === "agents" ? "active" : ""}`}>
              <div style={{ padding: "18px 16px", borderRadius: 18, background: "linear-gradient(135deg, #f9ede0 0%, #f7e3d8 100%)", border: "1px solid #e9c7b4", marginBottom: 16 }}>
                <div style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: 10, letterSpacing: "0.14em", textTransform: "uppercase", color: "var(--ps-accent-hover)", marginBottom: 8, fontWeight: 600 }}>
                  {lang === "zh" ? "助手中心" : "Agents"}
                </div>
                <div style={{ fontFamily: "'Noto Serif SC', 'Fraunces', serif", fontWeight: 500, fontSize: 18, lineHeight: 1.25, color: "var(--ps-ink-1)", marginBottom: 6 }}>
                  {lang === "zh" ? "让 AI 在你定的规矩里帮你办事。" : "Let AI handle things — within rules you set."}
                </div>
              </div>
              <div className="ps-section-label">{lang === "zh" ? "已接入 · 2 个助手" : "Connected · 2 agents"}</div>
              {[
                { name: "Claude Desktop", rule: { zh: "研究+支付订阅", en: "Research + pay subs" }, usage: "$42 / $500" },
                { name: "Cowork", rule: { zh: "收益再平衡", en: "Yield rebalance" }, usage: "$0 / $1,000" },
              ].map((a, i) => (
                <div key={i} style={{ background: "#fff", border: "1px solid var(--ps-line)", borderRadius: 16, padding: 14, marginBottom: 10 }}>
                  <div style={{ fontFamily: "'Noto Serif SC', 'Fraunces', serif", fontSize: 15, fontWeight: 500, marginBottom: 4 }}>{a.name}</div>
                  <div style={{ fontSize: 11, color: "var(--ps-ink-3)", marginBottom: 8 }}>{a.rule[lang]}</div>
                  <div style={{ fontSize: 11, fontFamily: "'JetBrains Mono', monospace", color: "var(--ps-ink-1)" }}>{a.usage}</div>
                </div>
              ))}
            </div>

            {/* Settings View */}
            <div className={`ps-view ${state.currentView === "settings" ? "active" : ""}`}>
              <div className="ps-section-label">{lang === "zh" ? "安全" : "Security"}</div>
              <button onClick={() => setView("keys")} style={{ display: "block", width: "100%", padding: 16, background: "#fff", border: "1px solid var(--ps-line)", borderRadius: 18, cursor: "pointer", textAlign: "left", fontFamily: "inherit", marginBottom: 14 }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div>
                    <div style={{ fontFamily: "'Noto Serif SC', 'Fraunces', serif", fontSize: 16, fontWeight: 500 }}>{lang === "zh" ? "三份钥匙体检" : "Three-piece key checkup"}</div>
                    <div style={{ fontSize: 12, color: "var(--ps-ink-3)", marginTop: 2 }}>{lang === "zh" ? "动你的钱，要三份钥匙都点头" : "All three must agree"}</div>
                  </div>
                  <span className="ps-chip ps-chip-green"><span style={{ width: 5, height: 5, borderRadius: "50%", background: "var(--ps-success)" }} />{lang === "zh" ? "都在" : "safe"}</span>
                </div>
              </button>
              <div className="ps-section-label">{lang === "zh" ? "一般" : "General"}</div>
              <div style={{ background: "#fff", border: "1px solid var(--ps-line)", borderRadius: 16, overflow: "hidden" }}>
                <div style={{ padding: "12px 14px", borderBottom: "1px solid var(--ps-line)", fontSize: 13 }}>{lang === "zh" ? "语言" : "Language"}</div>
                <button onClick={() => { clearMessages(); setView("home"); setOnboardingStage("hero"); }} style={{ display: "block", width: "100%", padding: "12px 14px", fontSize: 13, textAlign: "left", cursor: "pointer", background: "transparent", border: "none", fontFamily: "inherit" }}>
                  {lang === "zh" ? "重置引导流程" : "Redo onboarding"}
                </button>
              </div>
            </div>

            {/* Keys View */}
            <div className={`ps-view ${state.currentView === "keys" ? "active" : ""}`}>
              <button onClick={() => setView("settings")} style={{ display: "flex", alignItems: "center", gap: 6, background: "none", border: "none", cursor: "pointer", fontFamily: "inherit", fontSize: 13, color: "var(--ps-ink-2)", padding: "8px 0", marginBottom: 8 }}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M15 18l-6-6 6-6"/></svg>
                {lang === "zh" ? "返回" : "Back"}
              </button>
              <h1 style={{ fontFamily: "'Noto Serif SC', 'Fraunces', serif", fontWeight: 500, fontSize: 22, lineHeight: 1.2, marginBottom: 8 }}>
                {lang === "zh" ? "三份钥匙" : "Three pieces,"}
                <br />
                <span>{lang === "zh" ? "都点头才能" : "all must agree to "}</span>
                <em style={{ fontFamily: "'Fraunces', serif", fontStyle: "italic", color: "var(--ps-accent)" }}>{lang === "zh" ? "动钱" : "move"}</em>
              </h1>
              <p style={{ fontSize: 13, color: "var(--ps-ink-2)", marginBottom: 16 }}>
                {lang === "zh" ? "没人能单独动你的钱——连 cowallet 都进不来。" : "Nobody can move your money alone."}
              </p>
              {[
                { title: { zh: "手机里那份", en: "On this phone" }, status: "OK", healthy: true },
                { title: { zh: "云端那份", en: "In the cloud" }, status: "OK", healthy: true },
                { title: { zh: "找回码那份", en: "Recovery piece" }, status: "!", healthy: false },
              ].map((k, i) => (
                <div key={i} style={{ padding: 14, borderRadius: 14, border: `1px solid ${k.healthy ? "#c9d7bc" : "#e4d2a8"}`, background: k.healthy ? "var(--ps-success-soft)" : "var(--ps-warn-soft)", marginBottom: 8 }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <span style={{ fontFamily: "'Noto Serif SC', 'Fraunces', serif", fontSize: 14, fontWeight: 500 }}>{k.title[lang]}</span>
                    <span className={`ps-chip ${k.healthy ? "ps-chip-green" : "ps-chip-amber"}`}>{k.status}</span>
                  </div>
                </div>
              ))}
            </div>

            {/* Chat View */}
            <div className={`ps-view ${state.currentView === "chat" ? "active" : ""}`}>
              {state.chatMessages.length === 0 ? (
                <div style={{ textAlign: "center", padding: "40px 10px" }}>
                  <div style={{ width: 60, height: 60, borderRadius: "50%", background: "radial-gradient(circle at 35% 30%, #ffd4bc 0%, #D97757 45%, #8a3f2a 100%)", margin: "0 auto 16px" }} />
                  <div style={{ fontFamily: "'Noto Serif SC', 'Fraunces', serif", fontSize: 20, fontWeight: 500, marginBottom: 6 }}>
                    {lang === "zh" ? "说点什么?" : "What's on your mind?"}
                  </div>
                  <div style={{ fontSize: 13, color: "var(--ps-ink-3)", marginBottom: 20 }}>
                    {lang === "zh" ? "说话、打字都行。" : "Talk or type."}
                  </div>
                  {[
                    { zh: "我的钱都放哪了?", en: "Where's my money?" },
                    { zh: "帮我把稳定币存起来赚利息", en: "Park my stablecoins and earn" },
                  ].map((s, i) => (
                    <button key={i} onClick={() => sendMessage(s[lang])} style={{ display: "block", width: "100%", padding: "11px 14px", background: "#fff", border: "1px solid var(--ps-line)", borderRadius: 999, fontSize: 13, cursor: "pointer", marginBottom: 8, fontFamily: "inherit", color: "var(--ps-ink-1)" }}>
                      {s[lang]}
                    </button>
                  ))}
                </div>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: 10, paddingTop: 10 }}>
                  {state.chatMessages.map((msg) => (
                    <div key={msg.id}>
                      {msg.type === "user" ? (
                        <div className="ps-msg-user">{msg.text}</div>
                      ) : (
                        <div className="ps-msg-ai">
                          <div className="who">cowallet</div>
                          <div className="body">{msg.text}</div>
                          {msg.intent && (
                            <div className="ps-intent-card">
                              <div className="hd">{lang === "zh" ? "我听到的是" : "What I'm hearing"}</div>
                              <h4>{msg.intent.title[lang]}</h4>
                              <div className="sub">{msg.intent.sub[lang]}</div>
                              <div className="actions">
                                <button className="yes-btn" onClick={handleIntentYes}>{msg.intent.yes[lang]}</button>
                                <button className="no-btn" onClick={handleIntentNo}>{msg.intent.no[lang]}</button>
                              </div>
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Composer */}
          {(state.currentView === "home" || state.currentView === "chat") && (
            <div className="ps-composer">
              <div className="ps-composer-row">
                <input
                  ref={inputRef}
                  type="text"
                  value={inputValue}
                  onChange={(e) => setInputValue(e.target.value)}
                  onKeyDown={(e) => { if (e.key === "Enter") sendMessage(inputValue); }}
                  placeholder={lang === "zh" ? "跟 cowallet 说点什么…" : "Tell cowallet what you need…"}
                />
                <button className="ps-send-btn" onClick={() => sendMessage(inputValue)}>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z"/></svg>
                </button>
              </div>
            </div>
          )}

          {/* Tab bar */}
          <div className="ps-tabbar">
            <button className={`ps-tabbar-item ${state.currentView === "home" ? "active" : ""}`} onClick={() => setView("home")}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeLinejoin="round"><path d="M3 11l9-8 9 8v10a2 2 0 0 1-2 2h-4v-7h-6v7H5a2 2 0 0 1-2-2V11z"/></svg>
              <span>{lang === "zh" ? "首页" : "Home"}</span>
            </button>
            <button className={`ps-tabbar-item ${state.currentView === "wallet" ? "active" : ""}`} onClick={() => setView("wallet")}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor"><rect x="3" y="7" width="18" height="12" rx="2"/><path d="M3 10h18"/></svg>
              <span>{lang === "zh" ? "钱包" : "Wallet"}</span>
            </button>
            <button className="ps-ask-pill" onClick={() => setView("chat")}>
              <div className="ps-ask-orb" />
              <div className="ps-ask-label">{lang === "zh" ? "问" : "ASK"}</div>
            </button>
            <button className={`ps-tabbar-item ${state.currentView === "agents" ? "active" : ""}`} onClick={() => setView("agents")}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeLinecap="round"><circle cx="12" cy="8" r="4"/><path d="M4 20c0-4 4-6 8-6s8 2 8 6"/></svg>
              <span>{lang === "zh" ? "助手" : "Agents"}</span>
            </button>
            <button className={`ps-tabbar-item ${state.currentView === "settings" ? "active" : ""}`} onClick={() => setView("settings")}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor"><circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1.2l2-1.5-2-3.5-2.4 1a7 7 0 0 0-2.1-1.2L14 3h-4l-.4 2.6a7 7 0 0 0-2.1 1.2l-2.4-1-2 3.5 2 1.5A7 7 0 0 0 5 12c0 .4 0 .8.1 1.2l-2 1.5 2 3.5 2.4-1c.6.5 1.3.9 2.1 1.2L10 21h4l.4-2.6a7 7 0 0 0 2.1-1.2l2.4 1 2-3.5-2-1.5c0-.4.1-.8.1-1.2z"/></svg>
              <span>{lang === "zh" ? "设置" : "Settings"}</span>
            </button>
          </div>
        </>
      )}
    </PhoneFrame>
  );
}

function CreatingProgress({ onComplete }: { onComplete: () => void }) {
  const [progress, setProgress] = useState(0);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const completedRef = useRef(false);

  useEffect(() => {
    timerRef.current = setInterval(() => {
      setProgress((p) => {
        const next = p + 6 + Math.random() * 12;
        if (next >= 100) {
          if (timerRef.current) clearInterval(timerRef.current);
          if (!completedRef.current) {
            completedRef.current = true;
            setTimeout(onComplete, 500);
          }
          return 100;
        }
        return next;
      });
    }, 240);
    return () => { if (timerRef.current) clearInterval(timerRef.current); };
  }, [onComplete]);

  return (
    <div style={{ width: "100%", marginTop: 20 }}>
      <div style={{ height: 4, background: "var(--ps-line)", borderRadius: 3, overflow: "hidden" }}>
        <div style={{ height: "100%", background: "var(--ps-accent)", borderRadius: 3, width: `${Math.min(progress, 100)}%`, transition: "width 0.2s" }} />
      </div>
      <div style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: 11, color: "var(--ps-ink-3)", marginTop: 6 }}>
        {Math.round(Math.min(progress, 100))}%
      </div>
    </div>
  );
}
