import type { Metadata } from "next";
import { LegalArticle } from "@/components/legal/LegalArticle";
import { supportContent } from "@/lib/legal";

export const metadata: Metadata = {
  title: "支持中心 · cowallet",
  description: "cowallet 支持中心 — Support and FAQ for the AI-native MPC crypto wallet.",
};

export default function SupportPage() {
  return <LegalArticle content={supportContent} />;
}
