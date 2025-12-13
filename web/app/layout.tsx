import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { Analytics } from "@vercel/analytics/next";
import "./globals.css";
import { cn } from "@/lib/utils";
import JsonLd from "@/components/JsonLd";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "VEX Framework | AI Agent Infrastructure for Acquisition",
  description: "Production-ready Rust framework for autonomous AI agents. Complete IP available for acquisition. 550k+ ops/sec, cryptographic auditing, zero-copy architecture. By ProvnAI.",
  keywords: ["AI agents", "Rust framework", "autonomous AI", "agent infrastructure", "LLM framework", "AI acquisition", "ProvnAI", "VEX"],
  authors: [{ name: "ProvnAI", url: "https://www.provnai.com" }],
  creator: "ProvnAI",
  publisher: "ProvnAI",
  metadataBase: new URL("https://www.provnai.com"),
  alternates: {
    canonical: "/",
  },
  openGraph: {
    title: "VEX Framework | Production AI Agent Infrastructure for Acquisition",
    description: "Own the future of AI agents. Production-ready Rust infrastructure with 550k+ ops/sec, cryptographic auditing, and zero-copy architecture. Full IP available for acquisition.",
    type: "website",
    url: "https://www.provnai.com",
    siteName: "VEX Framework by ProvnAI",
    locale: "en_US",
    images: [
      {
        url: "/og-image.svg",
        width: 1200,
        height: 630,
        alt: "VEX Framework - AI Agent Infrastructure",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "VEX Framework | AI Agent Infrastructure for Acquisition",
    description: "Production-ready Rust framework for autonomous AI agents. Full IP available for acquisition.",
    site: "@provnai",
    creator: "@provnai",
    images: ["/og-image.svg"],
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      "max-video-preview": -1,
      "max-image-preview": "large",
      "max-snippet": -1,
    },
  },
  verification: {
    // Add your verification tokens here when ready
    // google: "your-google-verification-code",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark" suppressHydrationWarning>
      <head>
        <JsonLd />
      </head>
      <body className={cn(inter.className, "bg-background text-white antialiased")}>
        {children}
        <Analytics />
      </body>
    </html>
  );
}
