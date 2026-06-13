import type { Metadata } from "next";
import "./globals.css";
import { LangProvider } from "@/context/LangContext";

export const metadata: Metadata = {
  title: "cowallet · the wallet that reads you back",
  description: "会听懂人话的钱包 — AI-native MPC crypto wallet",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="zh" suppressHydrationWarning>
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
        <link
          href="https://fonts.googleapis.com/css2?family=Fraunces:opsz,ital,wght@9..144,0,400;9..144,0,500;9..144,0,600;9..144,1,400;9..144,1,500&family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&family=Noto+Serif+SC:wght@400;500;600;700&display=swap"
          rel="stylesheet"
        />
      </head>
      <body>
        <LangProvider>
          {children}
        </LangProvider>
      </body>
    </html>
  );
}
