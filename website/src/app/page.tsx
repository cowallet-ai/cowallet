"use client";

import { Navbar } from "@/components/sections/Navbar";
import { Hero } from "@/components/sections/Hero";
import { Features } from "@/components/sections/Features";
import { AiShowcase } from "@/components/sections/AiShowcase";
import { Security } from "@/components/sections/Security";
import { Protocols } from "@/components/sections/Protocols";
import { Cta } from "@/components/sections/Cta";
import { Footer } from "@/components/sections/Footer";

export default function Home() {
  return (
    <>
      <Navbar />
      <main>
        <Hero />
        <Features />
        <AiShowcase />
        <Security />
        <Protocols />
        <Cta />
      </main>
      <Footer />
    </>
  );
}
