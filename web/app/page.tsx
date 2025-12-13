import { Navbar } from "@/components/Navbar";
import { BentoGrid } from "@/components/BentoGrid";
import { Hero } from "@/components/Hero";
import { TrustBar } from "@/components/TrustBar";
import { Stats } from "@/components/Stats";
import { CodePreview } from "@/components/CodePreview";
import { TechStack } from "@/components/TechStack";
import { UseCases } from "@/components/UseCases";
import { Architecture } from "@/components/Architecture";
import { WhyAcquire } from "@/components/WhyAcquire";
import { Acquisition } from "@/components/Acquisition";
import { FAQ } from "@/components/FAQ";
import { Footer } from "@/components/Footer";
import { FloatingCTA } from "@/components/FloatingCTA";

export default function Home() {
  return (
    <main className="min-h-screen flex flex-col">
      <Navbar />
      <Hero />
      <TrustBar />
      <Stats />
      <BentoGrid />
      <UseCases />
      <CodePreview />
      <Architecture />
      <TechStack />
      <WhyAcquire />
      <FAQ />
      <Acquisition />
      <Footer />
      <FloatingCTA />
    </main>
  );
}
