export default function JsonLd() {
  const structuredData = {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    "name": "VEX Framework",
    "applicationCategory": "DeveloperApplication",
    "operatingSystem": "Cross-platform",
    "description": "Production-ready Rust framework for autonomous AI agents. Complete IP available for acquisition. 550k+ ops/sec, cryptographic auditing, zero-copy architecture.",
    "offers": {
      "@type": "Offer",
      "availability": "https://schema.org/InStock",
      "price": "0",
      "priceCurrency": "USD",
      "description": "Full IP Acquisition Available"
    },
    "author": {
      "@type": "Organization",
      "name": "ProvnAI",
      "url": "https://www.provnai.com"
    },
    "publisher": {
      "@type": "Organization",
      "name": "ProvnAI",
      "url": "https://www.provnai.com",
      "sameAs": [
        "https://x.com/provnai"
      ]
    },
    "aggregateRating": {
      "@type": "AggregateRating",
      "ratingValue": "5",
      "ratingCount": "1",
      "bestRating": "5"
    }
  };

  const organizationData = {
    "@context": "https://schema.org",
    "@type": "Organization",
    "name": "ProvnAI",
    "url": "https://www.provnai.com",
    "logo": "https://www.provnai.com/logo.png",
    "sameAs": [
      "https://x.com/provnai"
    ],
    "contactPoint": {
      "@type": "ContactPoint",
      "email": "info@moreclients.be",
      "contactType": "sales"
    }
  };

  return (
    <>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(structuredData) }}
      />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(organizationData) }}
      />
    </>
  );
}
