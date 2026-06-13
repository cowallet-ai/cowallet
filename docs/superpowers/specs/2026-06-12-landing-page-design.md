# cowallet Landing Page — Design Spec

Date: 2026-06-12

## Overview

Build a product landing page for cowallet using Next.js (App Router + Tailwind CSS), targeting potential users. The page introduces what cowallet is, its core value propositions, and guides visitors to download/sign up. The page embeds the existing interactive phone prototype as a live demo.

## Requirements

- **Audience:** Potential users (non-technical, crypto-curious)
- **Language:** Bilingual Chinese/English, default Chinese, toggle switch
- **Layout:** Single-page long scroll
- **Output:** Static export (`output: 'export'`), deployable to any CDN
- **Design:** Fully inherits prototype's visual language (paper palette, Claude orange, serif-led typography, orb animation)

## Project Structure

```
website/                          # Landing page project, sibling to backend/ and mobile/
├── next.config.ts                # output: 'export'
├── tailwind.config.ts            # Extended with prototype design tokens
├── package.json
├── public/
│   └── fonts/                    # Self-hosted: Fraunces, Inter, JetBrains Mono, Noto Serif SC
├── src/
│   ├── app/
│   │   ├── layout.tsx            # Global layout, font injection, LangContext provider
│   │   ├── page.tsx              # Landing page (single-page long scroll)
│   │   └── globals.css           # Tailwind base + CSS variables
│   ├── components/
│   │   ├── sections/             # Landing page sections
│   │   │   ├── Hero.tsx          # Hero + phone simulator
│   │   │   ├── Features.tsx      # Three core value propositions
│   │   │   ├── PhoneDemo.tsx     # Phone simulator wrapper (lazy loaded)
│   │   │   ├── AiShowcase.tsx    # AI capability showcase
│   │   │   ├── Security.tsx      # Three-key security model
│   │   │   ├── Protocols.tsx     # Developer protocols (brief)
│   │   │   ├── Cta.tsx           # Bottom CTA
│   │   │   └── Footer.tsx        # Page footer
│   │   ├── phone/                # Phone simulator internals
│   │   │   ├── PhoneFrame.tsx    # Phone shell (bezel, notch, home indicator)
│   │   │   ├── Onboarding.tsx    # 8-step onboarding flow
│   │   │   ├── HomeView.tsx      # Home view
│   │   │   ├── WalletView.tsx    # Wallet view
│   │   │   ├── AgentsView.tsx    # Agents view
│   │   │   ├── ChatView.tsx      # Chat view with intent recognition
│   │   │   ├── SettingsView.tsx  # Settings view
│   │   │   ├── KeysView.tsx      # Keys detail view
│   │   │   └── DemoController.tsx # Auto-demo orchestrator
│   │   └── ui/                   # Shared UI components
│   │       ├── Orb.tsx           # Signature orange orb (SVG + animation)
│   │       ├── LangToggle.tsx    # EN/中 toggle
│   │       └── Button.tsx        # Button variants (primary, accent, outline, ghost)
│   ├── context/
│   │   └── LangContext.tsx       # Language state (React Context)
│   ├── hooks/
│   │   └── usePhoneState.ts     # Phone simulator internal state
│   ├── lib/
│   │   ├── i18n.ts              # Bilingual copy (centralized)
│   │   └── intents.ts           # Intent detection regex rules
│   └── styles/
│       └── phone.css            # Phone simulator styles (extracted from prototype)
└── tsconfig.json
```

## Page Content Flow

### 1. Navbar (fixed top)
- Left: cowallet.ai logo + mini orb
- Right: LangToggle + "Download" CTA button
- On scroll: backdrop-blur glass effect

### 2. Hero
- Desktop: left text (headline + subtitle + CTA buttons) / right phone simulator
- Mobile: stacked — headline above, phone simulator below (scaled down)
- Background: radial-gradient warm paper texture (from prototype)
- Headline: "会听你说话的钱包" / "A wallet that actually listens"
- Sub: one-sentence explanation (from prototype hero)
- CTA: "开始使用" primary + "了解更多" secondary (scrolls down)

### 3. Features (three columns)
- Grid: 3 columns on desktop, stack on mobile
- Each: icon (colored square, matching prototype) + title + description
- Content:
  1. "只有你能动你的钱" — MPC security, not even cowallet can access
  2. "100+ 个金融网络" — EVM chains, global coverage
  3. "AI 帮你跑腿" — Natural language, voice, intent recognition

### 4. AI Showcase
- Alternating left-right layout (image + text)
- Three scenarios:
  1. AI reads contracts — shows intent card UI
  2. Voice/NL transfer — "send $100 to Sarah" flow
  3. AI auto-pay — agent with rules, budget limits
- Each with a static phone mockup built as a React component (same PhoneFrame shell + a frozen view state inside, not an actual image file)

### 5. Security (Three Keys)
- Centered headline: "三份钥匙都点头才能动钱"
- Three cards in a row: Phone / Cloud / Recovery
- Each card: icon + title + location + status badge
- Below: plain-language explanation (the "three locks on your door" metaphor)
- Tech detail card for pros: "MPC 2-of-3, Secure Enclave, Anchorage"

### 6. Protocols (brief)
- Section title: "开发者接口"
- List of protocols: MCP, x402, REST, Webhook, EIP-6963 — each with name + one-line desc + LIVE badge
- A2A marked as "soon"
- Bottom: "查看文档" link (placeholder href)

### 7. CTA (bottom)
- Large headline: "准备好了吗？" / "Ready?"
- Download buttons: App Store + Google Play (placeholder)
- Orb decorative animation

### 8. Footer
- "cowallet · 2026"
- "会听懂人话的钱包" tagline
- Links: Privacy / Terms / GitHub (placeholder hrefs)
- LangToggle (secondary)

## Phone Simulator Design

### State Management
`usePhoneState` hook manages:
- `currentView`: home | wallet | agents | settings | keys | chat
- `onboardingStage`: hero | start | creating | importing | bio | name | ready | persona
- `showOnboarding`: boolean
- `userName`: string
- `chatMessages`: Message[]
- `attachedImg`: string | null

Language state comes from the shared `LangContext` (synced with landing page toggle).

### Style Isolation
- Phone simulator styles extracted from prototype into `phone.css`
- Wrapped in `@layer components` with `.phone-scope` ancestor selector
- Landing page sections use only Tailwind utility classes
- No naming conflicts between the two

### Interaction Porting
- Onboarding: each stage is a sub-component, visibility controlled by state + CSS transitions
- Intent detection: regex rules in `intents.ts`, called from ChatView
- Auto-demo: `DemoController` uses async/await + timeouts to orchestrate, controls sub-components via refs
- Voice simulation: UI animation only, no actual recording
- Image upload: uses preset example image, no real file picker

### Performance
- Phone simulator marked `'use client'`
- Landing page sections remain Server Components (static content)
- Phone simulator loaded via `dynamic(() => import(...), { ssr: false })` — no SSR, lazy loaded
- SVG icons inlined (same as prototype)

## Design Tokens

### Colors (Tailwind extended)
| Token | Value | Usage |
|-------|-------|-------|
| paper | #faf9f5 | Main background |
| paper-subtle | #f1ead9 | Secondary background |
| paper-card | #ffffff | Card background |
| paper-hover | #efe7d3 | Hover state |
| ink-1 | #141008 | Primary text |
| ink-2 | #4a3f32 | Secondary text |
| ink-3 | #8a7a6c | Muted text |
| ink-4 | #b8a898 | Faintest text |
| line | #e7dfcd | Dividers |
| line-strong | #d5c9b0 | Strong dividers |
| accent | #D97757 | Primary accent (Claude orange) |
| accent-hover | #c96744 | Accent hover |
| accent-soft | #f7e3d8 | Accent background |
| danger | #c0392b | Error/danger |
| success | #5a7a4e | Success/healthy |
| warn | #b8832b | Warning |
| gold | #a88a4a | Gold/premium |
| info | #3d6b8c | Info/neutral |

### Typography
| Token | Stack |
|-------|-------|
| font-serif | "Noto Serif SC", "Fraunces", Georgia, serif |
| font-serif-en | "Fraunces", "Noto Serif SC", Georgia, serif |
| font-sans | "Inter", -apple-system, sans-serif |
| font-mono | "JetBrains Mono", ui-monospace, monospace |

### Border Radius
| Token | Value |
|-------|-------|
| card | 16px |
| btn | 14px |
| pill | 999px |
| phone | 48px |

### Animations
| Name | Spec |
|------|------|
| orb-breathe | 3.8s ease-in-out infinite, scale(1 → 1.04) |
| pulse-dot | 2.4s ease-in-out infinite, box-shadow pulse |
| view-in | 0.25s ease-out, opacity 0→1 + translateY(4px→0) |

## Responsive Strategy

| Breakpoint | Behavior |
|-----------|----------|
| Desktop (≥1024px) | Hero side-by-side, phone at actual size, 3-col grids |
| Tablet (768-1024px) | Hero stacked, phone slightly scaled down, 2-col grids |
| Mobile (<768px) | Full stack, phone further scaled, single column |

## Bilingual Implementation

- All user-facing copy lives in `src/lib/i18n.ts` as a structured object keyed by section/element
- `LangContext` provides `{ lang, setLang, t }` where `t(key)` resolves to current language string
- Language persisted in localStorage; defaults to `zh`
- Phone simulator and landing page share the same context — toggling language updates everywhere simultaneously

## Out of Scope

- Authentication / user accounts
- Actual App Store / Play Store links (placeholder)
- Backend API integration
- Real file upload in the phone simulator
- Real voice recording
- Analytics / tracking scripts
- SEO meta tags beyond basic title/description (can add later)

## Dependencies

```json
{
  "next": "^15",
  "react": "^19",
  "react-dom": "^19",
  "tailwindcss": "^4",
  "@tailwindcss/postcss": "^4"
}
```

No additional UI libraries, animation libraries, or icon packs. Keep it minimal — prototype's approach (inline SVG, CSS animations) is sufficient.
