"use client";

import { motion } from "framer-motion";

const crates = [
  {
    name: "vex-core",
    description: "Agent primitives, context management, evolution tracking, and Merkle tree audit infrastructure",
    lines: "~3,500",
    status: "Stable",
  },
  {
    name: "vex-llm",
    description: "Multi-provider LLM integration (OpenAI, Ollama, DeepSeek) with rate limiting and metrics",
    lines: "~2,200",
    status: "Stable",
  },
  {
    name: "vex-persist",
    description: "SQLite-based persistence with encryption, agent store, audit store, and job queue",
    lines: "~2,800",
    status: "Stable",
  },
  {
    name: "vex-runtime",
    description: "Async executor and orchestrator for multi-agent coordination and lifecycle management",
    lines: "~1,500",
    status: "Stable",
  },
  {
    name: "vex-temporal",
    description: "Time-decay memory systems with configurable decay curves and compression",
    lines: "~1,200",
    status: "Stable",
  },
  {
    name: "vex-adversarial",
    description: "Debate protocols, consensus mechanisms, and shadow verification for agent outputs",
    lines: "~1,800",
    status: "Stable",
  },
  {
    name: "vex-queue",
    description: "Job queue with persistent backend, worker pools, and priority scheduling",
    lines: "~1,100",
    status: "Stable",
  },
  {
    name: "vex-api",
    description: "HTTP API server with auth, rate limiting, circuit breakers, and telemetry",
    lines: "~1,900",
    status: "Stable",
  },
];

export function Architecture() {
  return (
    <section id="architecture" className="py-32 px-4">
      <div className="max-w-6xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="text-center mb-16 space-y-4"
        >
          <span className="text-primary text-sm font-medium uppercase tracking-wider">Codebase Structure</span>
          <h2 className="text-4xl md:text-5xl font-bold text-white">
            Modular <span className="text-primary">Crate</span> Architecture
          </h2>
          <p className="text-zinc-400 max-w-2xl mx-auto text-lg">
            Clean separation of concerns across 8 production crates. Each module is independently testable, documented, and ready for integration.
          </p>
        </motion.div>

        {/* Crates Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-16">
          {crates.map((crate, i) => (
            <motion.div
              key={crate.name}
              initial={{ opacity: 0, x: i % 2 === 0 ? -20 : 20 }}
              whileInView={{ opacity: 1, x: 0 }}
              viewport={{ once: true }}
              transition={{ delay: i * 0.05 }}
              whileHover={{ scale: 1.02 }}
              className="p-5 bg-zinc-900/50 border border-zinc-800 rounded-xl hover:border-zinc-700 transition-all"
            >
              <div className="flex items-start justify-between mb-2">
                <code className="text-primary font-mono font-semibold">{crate.name}</code>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-zinc-500 font-mono">{crate.lines} LOC</span>
                  <span className="px-2 py-0.5 text-xs bg-green-500/10 text-green-400 rounded-full">{crate.status}</span>
                </div>
              </div>
              <p className="text-sm text-zinc-400">{crate.description}</p>
            </motion.div>
          ))}
        </div>

        {/* Technical Highlights */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="grid grid-cols-1 md:grid-cols-3 gap-6"
        >
          <div className="p-6 bg-linear-to-br from-zinc-900 to-zinc-900/50 border border-zinc-800 rounded-2xl">
            <h3 className="text-lg font-semibold text-white mb-3">Dependencies</h3>
            <ul className="space-y-2 text-sm text-zinc-400">
              <li>• Tokio async runtime</li>
              <li>• Serde serialization</li>
              <li>• SQLx database layer</li>
              <li>• Axum HTTP framework</li>
              <li>• Reqwest HTTP client</li>
              <li>• SHA-256 cryptography</li>
            </ul>
          </div>
          
          <div className="p-6 bg-linear-to-br from-zinc-900 to-zinc-900/50 border border-zinc-800 rounded-2xl">
            <h3 className="text-lg font-semibold text-white mb-3">Quality Assurance</h3>
            <ul className="space-y-2 text-sm text-zinc-400">
              <li>• 94% test coverage</li>
              <li>• Zero unsafe blocks</li>
              <li>• Clippy lint clean</li>
              <li>• Comprehensive benchmarks</li>
              <li>• Integration test suite</li>
              <li>• CI/CD pipeline ready</li>
            </ul>
          </div>
          
          <div className="p-6 bg-linear-to-br from-zinc-900 to-zinc-900/50 border border-zinc-800 rounded-2xl">
            <h3 className="text-lg font-semibold text-white mb-3">Documentation</h3>
            <ul className="space-y-2 text-sm text-zinc-400">
              <li>• Full API documentation</li>
              <li>• Architecture decision records</li>
              <li>• Usage examples</li>
              <li>• Benchmark methodology</li>
              <li>• Deployment guides</li>
              <li>• Security considerations</li>
            </ul>
          </div>
        </motion.div>
      </div>
    </section>
  );
}
