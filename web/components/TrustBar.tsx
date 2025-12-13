"use client";

import { motion } from "framer-motion";
import { Shield, Zap, Lock, Award } from "lucide-react";

const badges = [
  { icon: Shield, label: "Memory Safe", sublabel: "Zero vulnerabilities" },
  { icon: Zap, label: "High Performance", sublabel: "550k+ ops/sec" },
  { icon: Lock, label: "Cryptographic Audit", sublabel: "Tamper-proof logs" },
  { icon: Award, label: "Production Ready", sublabel: "Battle-tested" },
];

export function TrustBar() {
  return (
    <section className="py-12 px-4 border-y border-zinc-800/50 bg-zinc-950/50">
      <div className="max-w-6xl mx-auto">
        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          className="flex flex-wrap justify-center items-center gap-8 md:gap-16"
        >
          {badges.map((badge, i) => (
            <motion.div
              key={badge.label}
              initial={{ opacity: 0, y: 10 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ delay: i * 0.1 }}
              className="flex items-center gap-3"
            >
              <div className="p-2 bg-zinc-900 rounded-lg border border-zinc-800">
                <badge.icon className="w-5 h-5 text-primary" />
              </div>
              <div>
                <div className="text-sm font-medium text-white">{badge.label}</div>
                <div className="text-xs text-zinc-500">{badge.sublabel}</div>
              </div>
            </motion.div>
          ))}
        </motion.div>
      </div>
    </section>
  );
}
