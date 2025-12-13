"use client";

import { motion } from "framer-motion";
import { Zap, Lock, Brain, Network, Database, Layers } from "lucide-react";
import { cn } from "@/lib/utils";

const features = [
  {
    title: "Zero-Copy Architecture",
    description: "High-performance memory management avoiding unnecessary data cloning. Achieves 550k+ ops/sec in benchmarks.",
    icon: Zap,
    className: "md:col-span-2",
    gradient: "from-yellow-500/20 to-orange-500/20",
  },
  {
    title: "Cryptographic Audit",
    description: "Every agent action is hashed and stored in a verifiable Merkle Tree for compliance.",
    icon: Lock,
    className: "md:col-span-1",
    gradient: "from-green-500/20 to-emerald-500/20",
  },
  {
    title: "Adversarial Verification",
    description: "Built-in debate protocols allow agents to challenge and verify each other's outputs.",
    icon: Network,
    className: "md:col-span-1",
    gradient: "from-purple-500/20 to-pink-500/20",
  },
  {
    title: "Temporal Memory",
    description: "Self-decaying context windows that optimize for relevance using sophisticated decay curves.",
    icon: Brain,
    className: "md:col-span-2",
    gradient: "from-blue-500/20 to-cyan-500/20",
  },
  {
    title: "SQLite + Encryption",
    description: "Secure, local-first persistence with optional at-rest encryption for sensitive state.",
    icon: Database,
    className: "md:col-span-1",
    gradient: "from-red-500/20 to-rose-500/20",
  },
  {
    title: "Modular Runtime",
    description: "Pluggable LLM providers (OpenAI, Ollama, DeepSeek), storage backends, and queue systems.",
    icon: Layers,
    className: "md:col-span-2",
    gradient: "from-indigo-500/20 to-violet-500/20",
  },
];

export function BentoGrid() {
  return (
    <section id="details" className="py-32 px-4 max-w-7xl mx-auto">
      <motion.div 
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        className="text-center mb-16 space-y-4"
      >
        <span className="text-primary text-sm font-medium uppercase tracking-wider">Technical Excellence</span>
        <h2 className="text-4xl md:text-5xl font-bold text-white">
          Production-Grade <span className="text-primary">Architecture</span>
        </h2>
        <p className="text-zinc-400 max-w-2xl mx-auto text-lg">
          Built for scale, security, and speed. Every component designed for real-world deployment.
        </p>
      </motion.div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {features.map((feature, i) => (
          <motion.div
            key={i}
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ delay: i * 0.1 }}
            whileHover={{ y: -5, transition: { duration: 0.2 } }}
            className={cn(
              "group relative overflow-hidden rounded-2xl bg-zinc-900 border border-zinc-800 p-8 hover:border-zinc-700 transition-all duration-300",
              feature.className
            )}
          >
            {/* Gradient overlay on hover */}
            <motion.div 
              className={cn(
                "absolute inset-0 bg-linear-to-br opacity-0 group-hover:opacity-100 transition-opacity duration-500",
                feature.gradient
              )}
            />
            
            {/* Glow effect */}
            <div className="absolute top-0 right-0 -mt-4 -mr-4 w-32 h-32 bg-primary/10 rounded-full blur-3xl group-hover:bg-primary/20 transition-all opacity-0 group-hover:opacity-100" />
            
            <div className="relative z-10 flex flex-col h-full justify-between">
              <motion.div 
                className="bg-zinc-800/50 w-fit p-3 rounded-xl mb-4 group-hover:bg-zinc-800 transition-colors"
                whileHover={{ scale: 1.1, rotate: 5 }}
              >
                <feature.icon className="w-6 h-6 text-primary" />
              </motion.div>
              
              <div>
                <h3 className="text-xl font-semibold text-white mb-2">{feature.title}</h3>
                <p className="text-zinc-400 leading-relaxed">{feature.description}</p>
              </div>
            </div>
          </motion.div>
        ))}
      </div>
    </section>
  );
}
