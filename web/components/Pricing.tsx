"use client";

import { motion } from "framer-motion";
import { Check, Zap, Building2, Rocket } from "lucide-react";
import Link from "next/link";

const tiers = [
  {
    name: "Open Source",
    price: "Free",
    description: "Perfect for individual developers and small projects",
    icon: Zap,
    features: [
      "Full core framework",
      "Community support",
      "MIT License",
      "Basic documentation",
      "GitHub discussions",
    ],
    cta: "Get Started",
    href: "https://github.com/provnai/vex",
    highlighted: false,
  },
  {
    name: "Enterprise",
    price: "Custom",
    description: "For teams that need dedicated support and SLAs",
    icon: Building2,
    features: [
      "Everything in Open Source",
      "Priority support (24h SLA)",
      "Custom integrations",
      "Security audit reports",
      "Private Slack channel",
      "Architecture review",
      "Training sessions",
    ],
    cta: "Contact Sales",
    href: "mailto:info@moreclients.be",
    highlighted: true,
  },
  {
    name: "Acquisition",
    price: "Inquire",
    description: "Full IP transfer and technology acquisition",
    icon: Rocket,
    features: [
      "Complete IP ownership",
      "All source code & assets",
      "Technical documentation",
      "Knowledge transfer",
      "Optional team acquisition",
      "Exclusivity agreement",
    ],
    cta: "Schedule Call",
    href: "https://www.provnai.com/contact",
    highlighted: false,
  },
];

export function Pricing() {
  return (
    <section className="py-24 px-4" id="pricing">
      <div className="max-w-6xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="text-center mb-16 space-y-4"
        >
          <h2 className="text-3xl md:text-4xl font-bold text-white">
            Simple, Transparent <span className="text-primary">Options</span>
          </h2>
          <p className="text-zinc-400 max-w-2xl mx-auto">
            Whether you&apos;re building a side project or acquiring technology for your enterprise, we have you covered.
          </p>
        </motion.div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {tiers.map((tier, i) => (
            <motion.div
              key={tier.name}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ delay: i * 0.1 }}
              className={`relative rounded-2xl p-8 ${
                tier.highlighted
                  ? "bg-linear-to-b from-primary/20 to-zinc-900 border-2 border-primary/50"
                  : "bg-zinc-900 border border-zinc-800"
              }`}
            >
              {tier.highlighted && (
                <div className="absolute -top-4 left-1/2 -translate-x-1/2 px-4 py-1 bg-primary text-white text-sm font-medium rounded-full">
                  Most Popular
                </div>
              )}

              <div className="flex items-center gap-3 mb-4">
                <div className={`p-2 rounded-lg ${tier.highlighted ? "bg-primary/20" : "bg-zinc-800"}`}>
                  <tier.icon className={`w-5 h-5 ${tier.highlighted ? "text-primary" : "text-zinc-400"}`} />
                </div>
                <h3 className="text-xl font-semibold text-white">{tier.name}</h3>
              </div>

              <div className="mb-4">
                <span className="text-4xl font-bold text-white">{tier.price}</span>
                {tier.price !== "Free" && tier.price !== "Inquire" && (
                  <span className="text-zinc-500 ml-2">/month</span>
                )}
              </div>

              <p className="text-zinc-400 mb-6">{tier.description}</p>

              <ul className="space-y-3 mb-8">
                {tier.features.map((feature) => (
                  <li key={feature} className="flex items-center gap-3 text-zinc-300">
                    <Check className={`w-4 h-4 ${tier.highlighted ? "text-primary" : "text-zinc-500"}`} />
                    {feature}
                  </li>
                ))}
              </ul>

              <Link
                href={tier.href}
                target={tier.href.startsWith("http") ? "_blank" : undefined}
                className={`block w-full py-3 px-4 rounded-xl font-medium text-center transition-all ${
                  tier.highlighted
                    ? "bg-primary hover:bg-primary/90 text-white"
                    : "bg-zinc-800 hover:bg-zinc-700 text-white"
                }`}
              >
                {tier.cta}
              </Link>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
