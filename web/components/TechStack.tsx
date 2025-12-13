"use client";

import { motion } from "framer-motion";

const technologies = [
  { name: "Rust", description: "Core Framework", color: "from-orange-500/20 to-red-500/20" },
  { name: "Tokio", description: "Async Runtime", color: "from-blue-500/20 to-cyan-500/20" },
  { name: "SQLite", description: "Persistence", color: "from-green-500/20 to-emerald-500/20" },
  { name: "OpenAI", description: "LLM Provider", color: "from-purple-500/20 to-pink-500/20" },
  { name: "Ollama", description: "Local Models", color: "from-yellow-500/20 to-orange-500/20" },
  { name: "DeepSeek", description: "Reasoning", color: "from-indigo-500/20 to-violet-500/20" },
];

export function TechStack() {
  return (
    <section className="py-20 px-4 overflow-hidden">
      <div className="max-w-6xl mx-auto">
        <motion.div 
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          className="text-center mb-12"
        >
          <span className="text-zinc-500 text-sm uppercase tracking-wider">
            Built With Modern Tech
          </span>
        </motion.div>

        <motion.div
          initial={{ opacity: 0 }}
          whileInView={{ opacity: 1 }}
          viewport={{ once: true }}
          className="flex flex-wrap justify-center gap-4"
        >
          {technologies.map((tech, i) => (
            <motion.div
              key={tech.name}
              initial={{ opacity: 0, scale: 0.9 }}
              whileInView={{ opacity: 1, scale: 1 }}
              viewport={{ once: true }}
              transition={{ delay: i * 0.05 }}
              whileHover={{ scale: 1.05, y: -2 }}
              className={`relative flex flex-col items-center gap-1 px-8 py-5 bg-zinc-900/50 border border-zinc-800 rounded-2xl hover:border-zinc-700 transition-all cursor-default overflow-hidden`}
            >
              <motion.div 
                className={`absolute inset-0 bg-linear-to-br ${tech.color} opacity-0 hover:opacity-100 transition-opacity`}
              />
              <span className="relative text-white font-semibold text-lg">{tech.name}</span>
              <span className="relative text-xs text-zinc-500">{tech.description}</span>
            </motion.div>
          ))}
        </motion.div>
      </div>
    </section>
  );
}
